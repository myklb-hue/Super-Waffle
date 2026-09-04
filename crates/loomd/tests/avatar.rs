//! Presence (SPEC §11).
//!
//! > Intent from the model, timing from the wires: a tool call sets what the
//! > face means, the speech audio sets when the mouth moves, the look port sets
//! > where it looks. None of the three waits for the others.
//!
//! That sentence is most of this file. The rest is §11.2's rule that the
//! vocabulary is generated from the rig, which is the custom-block rule applied
//! to animation: a model driving a Pixel is never offered an expression an
//! 8 × 8 matrix cannot make.

use loomd::run::event::RunEvent;
use loomd::run::model::{ChatTurn, Scripted};
use loomd::run::runner::{Bench, Decision, Runner, Warning};
use std::sync::Arc;

fn fixtures() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures"))
}

fn block(id: &str, kind: &str, settings: &[(&str, &str)]) -> graph_format::Block {
    graph_format::Block {
        id: id.into(),
        kind: kind.into(),
        title: None,
        position: graph_format::Position { x: 0.0, y: 0.0 },
        size: None,
        view: graph_format::View::Summary,
        settings: settings
            .iter()
            .map(|(k, v)| {
                (
                    (*k).to_owned(),
                    graph_format::Setting::String((*v).to_owned()),
                )
            })
            .collect(),
        ports: Vec::new(),
        source: None,
        disabled: false,
        breakpoint: false,
        frame: None,
    }
}

fn wire(id: &str, from: (&str, &str), to: (&str, &str)) -> graph_format::Wire {
    graph_format::Wire {
        id: id.into(),
        from: graph_format::Endpoint::new(from.0, from.1),
        to: graph_format::Endpoint::new(to.0, to.1),
    }
}

/// Figure 9's face: an orchestrator holding the Avatar's handle, an Affect
/// model on `express`, face recognition on `look`, speech fanned out to the
/// mouth.
fn with_a_face(rig: &str, wired: bool) -> graph_format::Graph {
    let mut graph = graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap();
    graph.run_mode = graph_format::RunMode::Once;
    graph.blocks = vec![
        block("ask", "keyboard", &[("placeholder", "hello")]),
        block("orchestrator", "llm", &[("role", "orchestrator")]),
        block("tts", "text-to-speech", &[]),
        block("face", "avatar", &[("rig", rig)]),
        block("out", "output", &[]),
    ];
    graph.wires = vec![
        wire("w1", ("ask", "text"), ("orchestrator", "prompt")),
        wire("w2", ("face", "tool"), ("orchestrator", "tools")),
        wire("w3", ("orchestrator", "text"), ("out", "value")),
    ];
    if wired {
        graph
            .wires
            .push(wire("w4", ("orchestrator", "text"), ("tts", "text")));
        graph
            .wires
            .push(wire("w5", ("tts", "audio"), ("face", "speech")));
    } else {
        // A text-to-speech with nothing wired to it is an error nobody asked
        // for, and this graph is not about speech.
        graph.blocks.retain(|b| b.id != "tts");
    }
    graph.frames.clear();
    graph
}

struct Ran {
    events: Vec<RunEvent>,
    provider: Scripted,
}

impl Ran {
    fn faces(&self) -> Vec<(String, f64, usize, Option<String>)> {
        self.events
            .iter()
            .filter_map(|e| match e {
                RunEvent::Face {
                    expression,
                    intensity,
                    mouth,
                    gaze,
                    ..
                } => Some((expression.clone(), *intensity, mouth.len(), gaze.clone())),
                _ => None,
            })
            .collect()
    }

    fn results(&self) -> Vec<String> {
        self.events
            .iter()
            .filter_map(|e| match e {
                RunEvent::ToolResult { result, .. } => Some(result.clone()),
                _ => None,
            })
            .collect()
    }

    /// The enum the model was actually offered for one tool.
    fn enum_for(&self, tool: &str, key: &str) -> Vec<String> {
        let seen = self.provider.seen.lock().unwrap();
        seen.iter()
            .flat_map(|r| r.tools.iter())
            .find(|t| t.name.ends_with(tool))
            .and_then(|t| {
                t.parameters
                    .get("properties")?
                    .get(key)?
                    .get("enum")?
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(str::to_owned))
                            .collect()
                    })
            })
            .unwrap_or_default()
    }
}

fn run(graph: &graph_format::Graph, turns: Vec<ChatTurn>) -> Ran {
    run_seeing(graph, turns, loomd::run::perceive::Scripted::default())
}

fn run_seeing(
    graph: &graph_format::Graph,
    turns: Vec<ChatTurn>,
    eye: loomd::run::perceive::Scripted,
) -> Ran {
    let provider = Scripted::new(turns);
    let root = fixtures();
    let mut events = Vec::new();
    Runner {
        graph,
        root: &root,
        provider: &provider,
        run: "avatar".into(),
        cancel: Default::default(),
        scratch: Arc::new(loomd::run::sense::Scratch::open("avatar-test").unwrap()),
        eye: Arc::new(eye),
        vault: Arc::new(loomd::run::memory::Vault::new(&root)),
        bench: Arc::new(Bench::scripted()),
    }
    .execute(&mut |e| events.push(e), &mut |_: &Warning| {
        Decision::Continue
    });
    Ran { events, provider }
}

/// §11.2: the vocabulary is generated from the rig. This is the custom-block
/// rule applied to animation — the interface is derived, never typed by hand.
#[test]
fn the_model_is_offered_what_the_rig_can_do_and_nothing_else() {
    let line = run(&with_a_face("line", false), vec![Scripted::says("hello")]);
    let words = line.enum_for("express", "emotion");
    assert!(words.contains(&"sleepy".to_owned()), "{words:?}");
    assert!(words.contains(&"love".to_owned()), "{words:?}");
    // Driven by the speech port, never by a command.
    assert!(!words.contains(&"speaking".to_owned()), "{words:?}");

    let pixel = run(&with_a_face("pixel", false), vec![Scripted::says("hello")]);
    let words = pixel.enum_for("express", "emotion");
    assert!(
        !words.contains(&"sleepy".to_owned()),
        "a matrix cannot sleep: {words:?}"
    );
    assert!(words.contains(&"smile".to_owned()), "{words:?}");
}

#[test]
fn gestures_are_generated_from_the_rig_too() {
    let line = run(&with_a_face("line", false), vec![Scripted::says("hi")]);
    assert_eq!(line.enum_for("gesture", "name"), ["nod", "shake"]);
    let orb = run(&with_a_face("orb", false), vec![Scripted::says("hi")]);
    assert!(
        orb.enum_for("gesture", "name").is_empty(),
        "an orb has no neck"
    );
}

/// §11.8: expressing is not a physical action and does not warn.
#[test]
fn a_tool_call_sets_the_face_without_asking() {
    let ran = run(
        &with_a_face("line", false),
        vec![
            Scripted::calls(
                "face.express",
                serde_json::json!({ "emotion": "smile", "intensity": 0.4 }),
            ),
            Scripted::says("there"),
        ],
    );
    let faces = ran.faces();
    assert!(
        faces
            .iter()
            .any(|(e, i, _, _)| e == "smile" && (*i - 0.4).abs() < 1e-9),
        "{faces:?}"
    );
}

/// A model may say anything, enum or not. "That face does not exist" is a
/// better answer than a drawing that does not.
#[test]
fn an_expression_the_rig_cannot_make_is_refused_in_words() {
    let ran = run(
        &with_a_face("pixel", false),
        vec![
            Scripted::calls("face.express", serde_json::json!({ "emotion": "sleepy" })),
            Scripted::says("sorry"),
        ],
    );
    let refusal = &ran.results()[0];
    assert!(refusal.contains("cannot look sleepy"), "{refusal}");
    assert!(
        refusal.contains("smile"),
        "it should say what it can do: {refusal}"
    );
}

#[test]
fn a_matrix_has_nowhere_to_look() {
    let ran = run(
        &with_a_face("pixel", false),
        vec![
            Scripted::calls("face.look", serde_json::json!({ "at": "Mykl" })),
            Scripted::says("ok"),
        ],
    );
    assert!(
        ran.results()[0].contains("nowhere to look"),
        "{:?}",
        ran.results()
    );

    let looking = run(
        &with_a_face("line", false),
        vec![
            Scripted::calls("face.look", serde_json::json!({ "at": "Mykl" })),
            Scripted::says("ok"),
        ],
    );
    assert!(
        looking
            .faces()
            .iter()
            .any(|(_, _, _, gaze)| gaze.as_deref() == Some("Mykl")),
        "{:?}",
        looking.faces()
    );
}

/// §11.3: lip sync never involves the model. The mouth is the shape of the
/// audio that is actually about to play, and it arrives on a wire.
#[test]
fn the_mouth_moves_from_the_speech_audio() {
    let ran = run(
        &with_a_face("line", true),
        vec![Scripted::says("hello there")],
    );
    let faces = ran.faces();
    let spoken = faces
        .iter()
        .find(|(_, _, mouth, _)| *mouth > 0)
        .unwrap_or_else(|| panic!("the mouth never opened: {faces:?}"));
    assert!(
        spoken.2 > 2,
        "the envelope should have a shape, not one bucket: {spoken:?}"
    );
    // And the expression is still whatever it was: speaking is a state, not a
    // command, and it did not replace the face.
    assert_eq!(spoken.0, "neutral");
}

/// The flow path (§11.3): expression as a value, for graphs that should not
/// spend a tool call on every smile.
#[test]
fn an_affect_wire_sets_the_face_without_a_tool_call() {
    let mut graph = with_a_face("line", false);
    graph.blocks.push(block("mood", "affect", &[]));
    graph
        .wires
        .push(wire("w6", ("ask", "text"), ("mood", "text")));
    graph
        .wires
        .push(wire("w7", ("mood", "affect"), ("face", "express")));
    let eye = loomd::run::perceive::Scripted {
        mood: Some(loomd::run::perceive::Affect {
            valence: 0.8,
            arousal: 0.3,
        }),
        ..Default::default()
    };
    let ran = run_seeing(&graph, vec![Scripted::says("hello")], eye);
    let faces = ran.faces();
    assert!(!faces.is_empty(), "the face never ran: {faces:?}");
    // The Affect block carries the expression, so the Avatar never has to know
    // where the thresholds are.
    assert_eq!(faces[0].0, "smile", "{faces:?}");
}
