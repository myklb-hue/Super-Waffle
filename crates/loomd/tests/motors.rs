//! Acting on the world, and what comes back (SPEC §6.6, §4.4, §9.1).
//!
//! > Motors are offered as a tool; a move warns first and then runs; the motors
//! > report position continuously and raise a fault that pauses the Toolbox.
//!
//! The acceptance for this slice is that a fault pauses the Toolbox and one
//! click resumes, so that is the shape of the last test. The controller is
//! scripted — there is no servo here — but everything on the engine's side is
//! real: the warning, the limits, the three feedback ports, the Toolbox that
//! stops taking calls and the `motor.home` that clears it.

use loomd::run::event::{Level, RunEvent};
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
                let value = match *v {
                    "true" => graph_format::Setting::Bool(true),
                    "false" => graph_format::Setting::Bool(false),
                    other => match other.parse::<i32>() {
                        Ok(n) => graph_format::Setting::Int(n),
                        Err(_) => graph_format::Setting::String(other.to_owned()),
                    },
                };
                ((*k).to_owned(), value)
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

/// Figure 9's motor chain, with the parts that are not about motors left out:
/// Motors `tool` into a Toolbox, the Toolbox into an orchestrator, `state` back
/// into its context, and `fault` into the Toolbox's `pause`.
fn motor_chain(pan_limit: &str, warn: &str) -> graph_format::Graph {
    let mut graph = graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap();
    graph.run_mode = graph_format::RunMode::Once;
    graph.blocks = vec![
        block("ask", "keyboard", &[("placeholder", "look at the door")]),
        block(
            "motors",
            "motors",
            &[("panLimit", pan_limit), ("warnBeforeMove", warn)],
        ),
        block("toolbox", "toolbox", &[]),
        block("orchestrator", "llm", &[("role", "orchestrator")]),
        block("out", "output", &[]),
    ];
    graph.wires = vec![
        wire("w1", ("ask", "text"), ("orchestrator", "prompt")),
        wire("w2", ("motors", "tool"), ("toolbox", "tools")),
        wire("w3", ("toolbox", "tools"), ("orchestrator", "tools")),
        wire("w4", ("motors", "state"), ("orchestrator", "context")),
        wire("w5", ("motors", "fault"), ("toolbox", "pause")),
        wire("w6", ("orchestrator", "text"), ("out", "value")),
    ];
    graph.frames.clear();
    graph
}

struct Ran {
    events: Vec<RunEvent>,
    warnings: Vec<Warning>,
    bench: Arc<Bench>,
}

impl Ran {
    fn console(&self) -> Vec<(Level, String)> {
        self.events
            .iter()
            .filter_map(|e| match e {
                RunEvent::Console { level, message, .. } => Some((*level, message.clone())),
                _ => None,
            })
            .collect()
    }

    fn said(&self, what: &str) -> bool {
        self.console().iter().any(|(_, m)| m.contains(what))
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

    /// What went out on a port, which is how telemetry is visible.
    fn on_port(&self, block: &str, port: &str) -> Vec<String> {
        self.events
            .iter()
            .filter_map(|e| match e {
                RunEvent::BlockOutput {
                    block: b,
                    port: p,
                    chunk,
                    ..
                } if b == block && p == port => Some(chunk.clone()),
                _ => None,
            })
            .collect()
    }
}

fn run(graph: &graph_format::Graph, turns: Vec<ChatTurn>, answer: Decision) -> Ran {
    let provider = Scripted::new(turns);
    let root = fixtures();
    let mut events = Vec::new();
    let mut warnings = Vec::new();
    let bench = Arc::new(Bench::scripted());
    Runner {
        graph,
        root: &root,
        provider: &provider,
        run: "motors".into(),
        cancel: Default::default(),
        scratch: Arc::new(loomd::run::sense::Scratch::open("motors-test").unwrap()),
        eye: Arc::new(loomd::run::perceive::Scripted::default()),
        vault: Arc::new(loomd::run::memory::Vault::new(&root)),
        bench: Arc::clone(&bench),
    }
    .execute(&mut |e| events.push(e), &mut |w| {
        warnings.push(w.clone());
        answer
    });
    Ran {
        events,
        warnings,
        bench,
    }
}

fn calls(name: &str, arguments: serde_json::Value) -> ChatTurn {
    Scripted::calls(name, arguments)
}

fn says(text: &str) -> ChatTurn {
    Scripted::says(text)
}

/// §13.3's narrative: the orchestrator calls `motor.move(pan: −40)` after a
/// warning.
#[test]
fn a_move_warns_first_and_then_goes() {
    let graph = motor_chain("90", "true");
    let ran = run(
        &graph,
        vec![
            calls("motors.move", serde_json::json!({ "pan": -40 })),
            says("the door is closed"),
        ],
        Decision::Continue,
    );
    assert_eq!(ran.warnings.len(), 1, "it moved without asking");
    assert!(
        ran.warnings[0].action.contains("-40"),
        "the warning did not say where: {:?}",
        ran.warnings[0]
    );
    assert!(
        ran.results().iter().any(|r| r.contains("pan -40")),
        "{:?}",
        ran.results()
    );
}

/// §12.1: the answer is the user's, and no is an answer.
#[test]
fn a_move_the_user_refuses_does_not_happen() {
    let graph = motor_chain("90", "true");
    let ran = run(
        &graph,
        vec![
            calls("motors.move", serde_json::json!({ "pan": -40 })),
            says("I have left it where it was"),
        ],
        Decision::Stop,
    );
    assert!(
        ran.results()
            .iter()
            .any(|r| r.contains("stopped before moving")),
        "{:?}",
        ran.results()
    );
    assert!(ran.on_port("motors", "state").is_empty(), "it moved anyway");
}

/// §4.4: telemetry is the second of the three shapes, and it goes out on
/// `state` where a wire can carry it into the orchestrator's context.
#[test]
fn a_move_reports_where_it_ended_up() {
    let graph = motor_chain("90", "false");
    let ran = run(
        &graph,
        vec![
            calls("motors.move", serde_json::json!({ "pan": -40, "tilt": 12 })),
            says("looking"),
        ],
        Decision::Continue,
    );
    assert_eq!(ran.on_port("motors", "state"), ["pan -40° · tilt 12°"]);
}

/// The acceptance: a fault pauses the Toolbox, and one click resumes.
///
/// Here the fault is a move past the pan limit, which is the honest kind — a
/// servo asked for an angle it does not have is a servo about to be broken, so
/// nothing moves and the block reports it exactly as a controller would at an
/// end stop.
#[test]
fn a_fault_stops_the_toolbox_and_home_starts_it_again() {
    let graph = motor_chain("30", "false");
    let ran = run(
        &graph,
        vec![
            // Past the limit: this faults.
            calls("motors.move", serde_json::json!({ "pan": -40 })),
            // Still stopped: the Toolbox is holding, so this does not go
            // through even though the angle is fine.
            calls("motors.move", serde_json::json!({ "pan": 10 })),
            // The clearing call, which §9.1 says always may.
            calls("motors.home", serde_json::json!({})),
            // And now it moves again.
            calls("motors.move", serde_json::json!({ "pan": 10 })),
            says("done"),
        ],
        Decision::Continue,
    );

    let results = ran.results();
    assert!(results[0].contains("past the 30° limit"), "{results:?}");
    assert!(
        results[1].contains("a fault stopped toolbox"),
        "the second call went through a stopped Toolbox: {results:?}"
    );
    assert!(
        results[2].contains("pan 0°"),
        "home did not go through: {results:?}"
    );
    assert!(
        results[3].contains("pan 10°"),
        "the Toolbox never started again: {results:?}"
    );

    // The fault fired on its own port, and the console said so in both places.
    assert_eq!(ran.on_port("motors", "fault").len(), 1);
    assert!(ran.said("fault: pan -40° is past the 30° limit"));
    assert!(ran.said("toolbox is holding"));
    assert!(ran.said("home cleared the fault"));
    assert!(
        ran.bench.holding().is_empty(),
        "still holding: {:?}",
        ran.bench.holding()
    );
}

/// §9.1: it pauses, it never locks. The clearing call is reachable from inside
/// the graph even while everything else is not.
#[test]
fn a_stopped_toolbox_is_not_a_locked_one() {
    let graph = motor_chain("30", "false");
    let ran = run(
        &graph,
        vec![
            calls("motors.move", serde_json::json!({ "pan": -40 })),
            calls("motors.home", serde_json::json!({})),
            says("recovered"),
        ],
        Decision::Continue,
    );
    assert!(
        ran.results()[1].contains("pan 0°"),
        "home was refused: {:?}",
        ran.results()
    );
}
