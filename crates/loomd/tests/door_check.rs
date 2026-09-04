//! SPEC §13.4's acceptance, run end to end.
//!
//! > A Python function with an `Image` parameter, a `float` default and a
//! > `Data` return becomes a block with one input port, one output port and a
//! > generated threshold slider. It is wired between a Webcam and a Notify.
//!
//! The Webcam is slice 7, so the frame is supplied directly rather than
//! captured. Everything between it and the Notify is real: the signature is
//! parsed, the function is called with the value on its port and the setting
//! from the inspector, and what it returns crosses a Convert on its way out.

use loomd::run::event::{RunEvent, RunOutcome};
use loomd::run::model::Scripted;
use loomd::run::runner::{Decision, Runner};
use loomd::run::value::Value;

fn fixtures() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures"))
}

/// The door-watch graph with the Webcam replaced by an Input, because a camera
/// is slice 7 and the block under test is the custom one.
fn graph() -> graph_format::Graph {
    let mut graph = graph_format::load(fixtures().join("graphs/door-watch.loom")).unwrap();
    let webcam = graph.blocks.iter_mut().find(|b| b.id == "webcam").unwrap();
    webcam.kind = "input".into();
    webcam.settings.clear();
    webcam.settings.insert(
        "value".into(),
        graph_format::Setting::String("a frame with a door in it".into()),
    );
    for wire in &mut graph.wires {
        if wire.from.node == "webcam" {
            wire.from.port = "value".into();
        }
    }
    // The fixture's code is §13.4's first version, which calls a `detect` the
    // file does not define. Give it one, which is what the user would have.
    let block = graph
        .blocks
        .iter_mut()
        .find(|b| b.id == "door-check")
        .unwrap();
    let source = block.source.as_mut().unwrap();
    source.code = Some(
        "def detect(frame):\n    return 0.9 if \"door\" in frame else 0.1\n\n\
         def door_check(frame: Image, threshold: float = 0.6) -> Data:\n\
         \x20   \"\"\"Is the front door open?\"\"\"\n\
         \x20   score = detect(frame)\n\
         \x20   return {\"open\": score > threshold, \"score\": score}\n"
            .to_owned(),
    );
    graph
}

fn run(graph: &graph_format::Graph) -> (loomd::run::runner::Summary, Vec<RunEvent>) {
    let provider = Scripted::new([]);
    let root = fixtures();
    let runner = Runner {
        graph,
        root: &root,
        provider: &provider,
        run: "door".into(),
        cancel: Default::default(),
        scratch: std::sync::Arc::new(loomd::run::sense::Scratch::open("t").unwrap()),
        eye: std::sync::Arc::new(loomd::run::perceive::Scripted::default()),
        vault: std::sync::Arc::new(loomd::run::memory::Vault::new("/tmp")),
    };
    let mut events = Vec::new();
    let summary = runner.execute(&mut |e| events.push(e), &mut |_| Decision::Continue);
    (summary, events)
}

#[test]
fn the_custom_block_runs_and_its_answer_reaches_the_end() {
    let graph = graph();
    let (summary, events) = run(&graph);

    // The run ends `Failed`, and the only thing that failed is the Notify at
    // the end of the chain: a desktop notification is an actuator and belongs
    // to a later slice. Asserting the exact reason is the point — a test that
    // dropped the Notify to get a green run would stop noticing the day this
    // fails for some other reason.
    assert_eq!(summary.outcome, RunOutcome::Failed);
    assert_eq!(summary.errors.len(), 1, "{:?}", summary.errors);
    assert!(
        summary.errors[0].starts_with("notify:"),
        "{:?}",
        summary.errors
    );
    assert!(summary.errors[0].contains("not a kind this engine can run yet"));

    // The function was called with the frame on its port and the threshold
    // from the inspector, and returned a record.
    let done = events
        .iter()
        .find_map(|e| match e {
            RunEvent::BlockDone { block, outputs, .. } if block == "door-check" => Some(outputs),
            _ => None,
        })
        .expect("the custom block finished");
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].port, "result");
    let Value::Data(record) = &done[0].value else {
        panic!("a Data return is a record, not {:?}", done[0].value);
    };
    assert_eq!(record["open"], serde_json::json!(true));
    assert_eq!(record["score"], serde_json::json!(0.9));

    // And the Convert between it and the Notify turned the record into text.
    let converted = events.iter().find_map(|e| match e {
        RunEvent::BlockDone { block, outputs, .. } if block == "to-text" => Some(outputs),
        _ => None,
    });
    let converted = converted.expect("the convert ran");
    assert_eq!(
        converted[0].value,
        Value::Text(r#"{"open":true,"score":0.9}"#.into())
    );
}

/// The threshold in the inspector is the one the function sees. This is the
/// whole point of a generated setting: changing it changes what the block does
/// without touching the code.
#[test]
fn the_generated_setting_changes_what_the_block_decides() {
    let mut graph = graph();
    graph
        .blocks
        .iter_mut()
        .find(|b| b.id == "door-check")
        .unwrap()
        .settings
        .insert("threshold".into(), graph_format::Setting::Float(0.95));

    let (_, events) = run(&graph);
    let done = events
        .iter()
        .find_map(|e| match e {
            RunEvent::BlockDone { block, outputs, .. } if block == "door-check" => Some(outputs),
            _ => None,
        })
        .unwrap();
    let Value::Data(record) = &done[0].value else {
        panic!("expected a record");
    };
    // 0.9 is no longer over the threshold, so the door reads as shut.
    assert_eq!(record["open"], serde_json::json!(false));
}

/// A block that throws shows the user their own error and does not take the
/// run down with it (SPEC §10.4, §12.1).
#[test]
fn an_error_in_the_users_code_is_reported_in_their_own_words() {
    let mut graph = graph();
    graph
        .blocks
        .iter_mut()
        .find(|b| b.id == "door-check")
        .unwrap()
        .source
        .as_mut()
        .unwrap()
        .code = Some(
        "def door_check(frame: Image, threshold: float = 0.6) -> Data:\n\
         \x20   raise ValueError('no door in this frame')\n"
            .to_owned(),
    );

    let (summary, events) = run(&graph);
    assert_eq!(summary.outcome, RunOutcome::Failed);
    let error = events
        .iter()
        .find_map(|e| match e {
            RunEvent::BlockError { block, message, .. } if block == "door-check" => Some(message),
            _ => None,
        })
        .expect("the block reported");
    assert!(error.contains("no door in this frame"), "{error}");
    // The run reached its end rather than stopping at the failure.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RunEvent::Finished { .. })),
        "the run still finished"
    );
}

/// A syntax error is reported with its line number while the graph carries on
/// around it (SPEC §10.4).
#[test]
fn a_syntax_error_names_its_line() {
    let mut graph = graph();
    graph
        .blocks
        .iter_mut()
        .find(|b| b.id == "door-check")
        .unwrap()
        .source
        .as_mut()
        .unwrap()
        .code = Some("x = 1\n\ndef door_check(frame: Image,\n".to_owned());

    let (summary, events) = run(&graph);
    assert_eq!(summary.outcome, RunOutcome::Failed);
    let error = events
        .iter()
        .find_map(|e| match e {
            RunEvent::BlockError { block, message, .. } if block == "door-check" => Some(message),
            _ => None,
        })
        .unwrap();
    assert!(error.starts_with("line 3:"), "{error}");
}

/// What the function printed reaches the console, beside every other line the
/// graph produced.
#[test]
fn a_print_in_the_users_code_reaches_the_console() {
    let mut graph = graph();
    graph
        .blocks
        .iter_mut()
        .find(|b| b.id == "door-check")
        .unwrap()
        .source
        .as_mut()
        .unwrap()
        .code = Some(
        "def door_check(frame: Image, threshold: float = 0.6) -> Data:\n\
         \x20   print('checking the frame')\n\
         \x20   return {\"open\": True}\n"
            .to_owned(),
    );

    let (_, events) = run(&graph);
    let said: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            RunEvent::Console {
                source, message, ..
            } if source.as_deref() == Some("door-check") => Some(message.as_str()),
            _ => None,
        })
        .collect();
    assert!(said.contains(&"checking the frame"), "{said:?}");
}
