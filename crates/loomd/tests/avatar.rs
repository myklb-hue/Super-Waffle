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

// ------------------------------------------------------------ what a face
// carries, and does, beyond its expression

impl Ran {
    /// Every face event, whole.
    fn face_events(&self) -> Vec<&RunEvent> {
        self.events
            .iter()
            .filter(|e| matches!(e, RunEvent::Face { .. }))
            .collect()
    }

    fn console(&self) -> Vec<String> {
        self.events
            .iter()
            .filter_map(|e| match e {
                RunEvent::Console { message, .. } => Some(message.clone()),
                _ => None,
            })
            .collect()
    }
}

/// A gesture is a one-shot: it rides on the event that carries it and is not
/// part of the face's state, so the next event does not nod again.
#[test]
fn a_gesture_rides_on_one_event_and_is_not_kept() {
    let ran = run(
        &with_a_face("line", false),
        vec![
            Scripted::calls("face.gesture", serde_json::json!({ "name": "nod" })),
            Scripted::calls("face.express", serde_json::json!({ "emotion": "smile" })),
            Scripted::says("done"),
        ],
    );
    let gestures: Vec<Option<String>> = ran
        .face_events()
        .iter()
        .map(|e| match e {
            RunEvent::Face { gesture, .. } => gesture.clone(),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(gestures, [Some("nod".to_owned()), None], "{gestures:?}");
    assert!(ran.results()[0].contains("nod"), "{:?}", ran.results());
}

/// "gaze to a point or a person" (SPEC §11.2): two numbers are a place, and
/// the place travels with the words.
#[test]
fn a_look_at_a_place_carries_the_place() {
    let ran = run(
        &with_a_face("line", false),
        vec![
            Scripted::calls("face.look", serde_json::json!({ "at": "0.25, 0.75" })),
            Scripted::calls("face.look", serde_json::json!({ "at": "Mykl" })),
            Scripted::says("ok"),
        ],
    );
    let looks: Vec<(Option<String>, Option<[f64; 2]>)> = ran
        .face_events()
        .iter()
        .map(|e| match e {
            RunEvent::Face { gaze, gaze_at, .. } => (gaze.clone(), *gaze_at),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(looks[0], (Some("0.25, 0.75".into()), Some([0.25, 0.75])));
    // A name is a name; where the person is, is the shell's to decide.
    assert_eq!(looks[1], (Some("Mykl".into()), None));
}

/// Auto-affect from speech: the face wears the mood of what it is saying,
/// read from the words the voice remembered, with no tool call and no Affect
/// block. Off is off.
#[test]
fn the_face_wears_the_mood_of_what_it_says() {
    let happy = || loomd::run::perceive::Scripted {
        mood: Some(loomd::run::perceive::Affect {
            valence: 0.8,
            arousal: 0.3,
        }),
        ..Default::default()
    };
    let ran = run_seeing(
        &with_a_face("line", true),
        vec![Scripted::says("what a lovely morning")],
        happy(),
    );
    let faces = ran.faces();
    let spoken = faces
        .iter()
        .find(|(_, _, mouth, _)| *mouth > 0)
        .unwrap_or_else(|| panic!("the mouth never opened: {faces:?}"));
    assert_eq!(spoken.0, "smile", "{faces:?}");

    let mut off = with_a_face("line", true);
    off.blocks
        .iter_mut()
        .find(|b| b.id == "face")
        .unwrap()
        .settings
        .insert(
            "autoAffectFromSpeech".into(),
            graph_format::Setting::Bool(false),
        );
    let ran = run_seeing(&off, vec![Scripted::says("what a lovely morning")], happy());
    let faces = ran.faces();
    assert!(
        faces.iter().all(|(e, _, _, _)| e == "neutral"),
        "with the switch off the mouth moves and the face does not: {faces:?}"
    );
}

/// An Affect block wired to `express` is the alternative the setting names,
/// so auto-affect stands aside for it even when the switch is on.
#[test]
fn a_wired_affect_block_is_asked_and_auto_affect_stands_aside() {
    let mut graph = with_a_face("line", true);
    graph.blocks.push(block("mood", "affect", &[]));
    graph
        .wires
        .push(wire("w6", ("ask", "text"), ("mood", "text")));
    graph
        .wires
        .push(wire("w7", ("mood", "affect"), ("face", "express")));
    let eye = loomd::run::perceive::Scripted {
        mood: Some(loomd::run::perceive::Affect {
            valence: -0.8,
            arousal: 0.3,
        }),
        ..Default::default()
    };
    let ran = run_seeing(&graph, vec![Scripted::says("hello")], eye);
    // One affect question for the run: the Affect block's, about the prompt.
    // The Avatar did not ask a second time about the speech.
    let seen = ran.provider.seen.lock().unwrap().len();
    assert!(seen >= 1);
    let faces = ran.faces();
    assert!(faces.iter().all(|(e, _, _, _)| e == "frown"), "{faces:?}");
}

/// The event carries the idle the shell should animate: from the rig, unless
/// the block says otherwise.
#[test]
fn the_event_carries_the_idle_numbers() {
    let ran = run(
        &with_a_face("line", false),
        vec![
            Scripted::calls("face.express", serde_json::json!({ "emotion": "smile" })),
            Scripted::says("ok"),
        ],
    );
    let RunEvent::Face {
        blink_ms,
        breathe_per_min,
        colour,
        asleep,
        ..
    } = ran.face_events()[0]
    else {
        unreachable!()
    };
    assert_eq!(*blink_ms, 4000);
    assert_eq!(*breathe_per_min, 13);
    assert_eq!(colour, "#6fc98a");
    assert!(!asleep);

    let mut still = with_a_face("line", false);
    still
        .blocks
        .iter_mut()
        .find(|b| b.id == "face")
        .unwrap()
        .settings
        .insert("breathePerMin".into(), graph_format::Setting::Int(0));
    let ran = run(
        &still,
        vec![
            Scripted::calls("face.express", serde_json::json!({ "emotion": "smile" })),
            Scripted::says("ok"),
        ],
    );
    let RunEvent::Face {
        breathe_per_min, ..
    } = ran.face_events()[0]
    else {
        unreachable!()
    };
    assert_eq!(*breathe_per_min, 0);
}

/// Idle (SPEC §11.4): an expression settles, a quiet face sleeps, an event
/// wakes it. The clock is a parameter, so this does not wait five minutes.
#[test]
fn a_face_settles_then_sleeps_then_wakes() {
    use std::time::{Duration, Instant};
    let graph = with_a_face("line", false);
    let provider = Scripted::new(vec![
        Scripted::calls("face.express", serde_json::json!({ "emotion": "smile" })),
        Scripted::says("ok"),
    ]);
    let root = fixtures();
    let bench = Arc::new(Bench::scripted());
    let runner = Runner {
        graph: &graph,
        root: &root,
        provider: &provider,
        run: "idle".into(),
        cancel: Default::default(),
        scratch: Arc::new(loomd::run::sense::Scratch::open("avatar-idle").unwrap()),
        eye: Arc::new(loomd::run::perceive::Scripted::default()),
        vault: Arc::new(loomd::run::memory::Vault::new(&root)),
        bench: Arc::clone(&bench),
    };
    let mut events = Vec::new();
    runner.execute(&mut |e| events.push(e), &mut |_: &Warning| {
        Decision::Continue
    });
    let started = Instant::now();
    let last = |events: &Vec<RunEvent>| match events
        .iter()
        .rev()
        .find(|e| matches!(e, RunEvent::Face { .. }))
    {
        Some(RunEvent::Face {
            expression, asleep, ..
        }) => (expression.clone(), *asleep),
        _ => panic!("no face event"),
    };
    assert_eq!(last(&events), ("smile".into(), false));

    // Not yet: the settle timer is six seconds.
    let before = events.len();
    runner.idle_faces(started + Duration::from_secs(2), &mut |e| events.push(e));
    assert_eq!(events.len(), before, "nothing should have changed at 2 s");

    // Settled.
    runner.idle_faces(started + Duration::from_secs(7), &mut |e| events.push(e));
    assert_eq!(last(&events), ("neutral".into(), false));

    // Asleep, wearing the state the rig has for it.
    runner.idle_faces(started + Duration::from_secs(301), &mut |e| events.push(e));
    assert_eq!(last(&events), ("sleepy".into(), true));
    // And nothing more happens while it sleeps.
    let before = events.len();
    runner.idle_faces(started + Duration::from_secs(900), &mut |e| events.push(e));
    assert_eq!(events.len(), before);

    // An event. The next idle pass wakes it.
    bench.touch_at(started + Duration::from_secs(900) + Duration::from_millis(1));
    runner.idle_faces(started + Duration::from_secs(901), &mut |e| events.push(e));
    assert_eq!(last(&events), ("neutral".into(), false));
}

/// "surprised: a beat, then settles" — sooner than the settle timer.
/// And a rig that cannot look sleepy sleeps with its eyes open: the flag,
/// not the face.
#[test]
fn a_surprise_is_a_beat_and_a_matrix_sleeps_without_a_sleepy_face() {
    use std::time::{Duration, Instant};
    let graph = with_a_face("pixel", false);
    let provider = Scripted::new(vec![
        Scripted::calls(
            "face.express",
            serde_json::json!({ "emotion": "surprised" }),
        ),
        Scripted::says("ok"),
    ]);
    let root = fixtures();
    let runner = Runner {
        graph: &graph,
        root: &root,
        provider: &provider,
        run: "beat".into(),
        cancel: Default::default(),
        scratch: Arc::new(loomd::run::sense::Scratch::open("avatar-beat").unwrap()),
        eye: Arc::new(loomd::run::perceive::Scripted::default()),
        vault: Arc::new(loomd::run::memory::Vault::new(&root)),
        bench: Arc::new(Bench::scripted()),
    };
    let mut events = Vec::new();
    runner.execute(&mut |e| events.push(e), &mut |_: &Warning| {
        Decision::Continue
    });
    let started = Instant::now();
    let last = |events: &Vec<RunEvent>| match events
        .iter()
        .rev()
        .find(|e| matches!(e, RunEvent::Face { .. }))
    {
        Some(RunEvent::Face {
            expression, asleep, ..
        }) => (expression.clone(), *asleep),
        _ => panic!("no face event"),
    };
    assert_eq!(last(&events), ("surprised".into(), false));
    runner.idle_faces(started + Duration::from_secs(2), &mut |e| events.push(e));
    assert_eq!(
        last(&events),
        ("neutral".into(), false),
        "a beat is shorter than 2 s"
    );
    runner.idle_faces(started + Duration::from_secs(301), &mut |e| events.push(e));
    assert_eq!(last(&events), ("neutral".into(), true));
}

/// `face.render` (SPEC §11.5): the face goes to the USB device block the
/// Avatar names — as matrix bits for a matrix rig, as words for any other.
#[test]
fn the_face_renders_to_the_device_it_names() {
    let render_with = |rig: &str| {
        let mut graph = with_a_face(rig, false);
        graph
            .blocks
            .push(block("matrix", "usb-device", &[("port", "/dev/null")]));
        let face = graph.blocks.iter_mut().find(|b| b.id == "face").unwrap();
        face.settings.insert(
            "output".into(),
            graph_format::Setting::String("device".into()),
        );
        face.settings.insert(
            "device".into(),
            graph_format::Setting::String("matrix".into()),
        );
        let device = Arc::new(loomd::run::actuate::Scripted::default());
        let bench = Arc::new(Bench::scripted());
        bench.plant("matrix", device.clone());
        let provider = Scripted::new(vec![
            Scripted::calls("face.express", serde_json::json!({ "emotion": "love" })),
            Scripted::says("ok"),
        ]);
        let root = fixtures();
        let mut events = Vec::new();
        Runner {
            graph: &graph,
            root: &root,
            provider: &provider,
            run: "render".into(),
            cancel: Default::default(),
            scratch: Arc::new(loomd::run::sense::Scratch::open("avatar-render").unwrap()),
            eye: Arc::new(loomd::run::perceive::Scripted::default()),
            vault: Arc::new(loomd::run::memory::Vault::new(&root)),
            bench,
        }
        .execute(&mut |e| events.push(e), &mut |_: &Warning| {
            Decision::Continue
        });
        device.sent.lock().unwrap().clone()
    };
    let pixel = render_with("pixel");
    assert_eq!(pixel.len(), 1, "{pixel:?}");
    let mut words = pixel[0].split(' ');
    assert_eq!(words.next(), Some("face"));
    assert_eq!(words.next(), Some("love"));
    assert_eq!(words.next(), Some("1.00"));
    let bits = words.next().expect("a matrix rig sends its bits");
    assert_eq!(bits.len(), 16, "{bits}");
    assert!(
        bits.starts_with("66fc"),
        "a heart starts with two bumps: {bits}"
    );

    let line = render_with("line");
    assert_eq!(
        line,
        ["face love 1.00"],
        "a rig that is not a grid goes as words"
    );
}

/// The family (SPEC §11.7): a Status light breathes the mood's colour on its
/// device, on the same vocabulary as the face.
#[test]
fn a_status_light_tells_its_device_the_colour() {
    let mut graph = with_a_face("line", false);
    graph.blocks.retain(|b| b.id != "face");
    graph.wires.retain(|w| w.id != "w2");
    graph
        .blocks
        .push(block("lamp", "status-light", &[("device", "/dev/ttyUSB9")]));
    graph
        .wires
        .push(wire("w2", ("lamp", "tool"), ("orchestrator", "tools")));
    let device = Arc::new(loomd::run::actuate::Scripted::default());
    let bench = Arc::new(Bench::scripted());
    bench.plant("lamp", device.clone());
    let provider = Scripted::new(vec![
        Scripted::calls(
            "lamp.express",
            serde_json::json!({ "emotion": "smile", "intensity": 0.5 }),
        ),
        Scripted::says("ok"),
    ]);
    let root = fixtures();
    let mut events = Vec::new();
    Runner {
        graph: &graph,
        root: &root,
        provider: &provider,
        run: "lamp".into(),
        cancel: Default::default(),
        scratch: Arc::new(loomd::run::sense::Scratch::open("avatar-lamp").unwrap()),
        eye: Arc::new(loomd::run::perceive::Scripted::default()),
        vault: Arc::new(loomd::run::memory::Vault::new(&root)),
        bench,
    }
    .execute(&mut |e| events.push(e), &mut |_: &Warning| {
        Decision::Continue
    });
    let sent = device.sent.lock().unwrap().clone();
    assert_eq!(sent, ["light smile #6fc98a 0.50"]);
}

/// A Sound cue with a pack that has no file for a mood says so once and is
/// otherwise silent, rather than failing the turn.
#[test]
fn a_sound_cue_with_no_file_for_a_mood_says_so_once() {
    let pack = std::env::temp_dir().join(format!("cyberloom-pack-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&pack);
    let mut graph = with_a_face("line", false);
    graph.blocks.retain(|b| b.id != "face");
    graph.wires.retain(|w| w.id != "w2");
    graph.blocks.push(block(
        "chime",
        "sound-cue",
        &[("pack", pack.to_str().unwrap())],
    ));
    graph
        .wires
        .push(wire("w2", ("chime", "tool"), ("orchestrator", "tools")));
    let ran = run(
        &graph,
        vec![
            Scripted::calls("chime.express", serde_json::json!({ "emotion": "smile" })),
            Scripted::calls("chime.express", serde_json::json!({ "emotion": "smile" })),
            Scripted::says("ok"),
        ],
    );
    let silent = ran
        .console()
        .iter()
        .filter(|m| m.contains("no smile.wav"))
        .count();
    assert_eq!(silent, 1, "{:?}", ran.console());
    assert!(
        ran.results().iter().all(|r| r.contains("smile")),
        "{:?}",
        ran.results()
    );
    let _ = std::fs::remove_dir_all(&pack);
}
