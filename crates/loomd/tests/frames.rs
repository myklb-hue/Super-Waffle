//! A loop frame that repeats (SPEC §3.5, §8.3).
//!
//! > Blocks inside repeat once per item.
//!
//! Which is the whole of it, and the whole of what these check: that a list
//! becomes that many passes, that `parallel` really means at the same time,
//! that `max` is a ceiling, that continue-on-error is the difference between
//! losing one item and losing the batch, and that a block inside a frame that
//! warns asks once rather than two hundred times.

use loomd::run::event::{RunEvent, RunOutcome};
use loomd::run::model::Scripted;
use loomd::run::runner::{Decision, Runner, Summary, Warning};
use loomd::run::value::Value;

fn fixtures() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures"))
}

fn block(
    id: &str,
    kind: &str,
    frame: Option<&str>,
    settings: &[(&str, &str)],
) -> graph_format::Block {
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
                    other => graph_format::Setting::String(other.to_owned()),
                };
                ((*k).to_owned(), value)
            })
            .collect(),
        ports: Vec::new(),
        source: None,
        disabled: false,
        breakpoint: false,
        frame: frame.map(str::to_owned),
    }
}

/// A frame over a list, with one Terminal inside it that records what it saw.
fn looping(
    items: &str,
    command: &str,
    parallel: u32,
    max: u32,
    keep_going: bool,
) -> graph_format::Graph {
    let mut graph = graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap();
    graph.blocks = vec![
        block("input", "input", None, &[("value", items)]),
        block(
            "work",
            "terminal",
            Some("each"),
            &[("command", command), ("warnBefore", "false")],
        ),
    ];
    graph.frames = vec![graph_format::Frame {
        id: "each".into(),
        kind: graph_format::FrameKind::Loop,
        position: graph_format::Position { x: 0.0, y: 0.0 },
        size: graph_format::Size { w: 400.0, h: None },
        over: graph_format::Endpoint::new("input", "value"),
        as_name: "item".into(),
        parallel,
        max,
        stop_when: None,
        continue_on_error: keep_going,
    }];
    graph.wires = vec![graph_format::Wire {
        id: "w1".into(),
        from: graph_format::Endpoint::new("input", "value"),
        to: graph_format::Endpoint::new("each", "items"),
    }];
    graph
}

fn run(graph: &graph_format::Graph) -> (Summary, Vec<RunEvent>, Vec<Warning>) {
    let provider = Scripted::new([]);
    let root = fixtures();
    let mut events = Vec::new();
    let mut asked = Vec::new();
    let summary = Runner {
        graph,
        root: &root,
        provider: &provider,
        run: "frames".into(),
        cancel: Default::default(),
    }
    .execute(&mut |e| events.push(e), &mut |w| {
        asked.push(w.clone());
        Decision::Continue
    });
    (summary, events, asked)
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cyberloom-frame-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A list of three becomes three passes, and each pass sees its own item.
///
/// The item is wired into the Terminal's `text` port, which is what a Terminal
/// takes as its command — so the frame here iterates commands, and what lands
/// in the file says which items really ran and in what order.
#[test]
fn a_frame_runs_its_blocks_once_per_item() {
    let dir = scratch("each");
    let items = format!(
        r#"["echo alpha >> {0}/seen","echo beta >> {0}/seen","echo gamma >> {0}/seen"]"#,
        dir.display()
    );
    let mut graph = looping(&items, "true", 1, 100, true);
    graph.wires.push(graph_format::Wire {
        id: "w2".into(),
        from: graph_format::Endpoint::new("each", "item"),
        to: graph_format::Endpoint::new("work", "text"),
    });

    let (summary, events, _) = run(&graph);
    assert_eq!(
        summary.outcome,
        RunOutcome::Finished,
        "{:?}",
        summary.errors
    );

    let seen = std::fs::read_to_string(dir.join("seen")).unwrap();
    assert_eq!(
        seen.lines().collect::<Vec<_>>(),
        ["alpha", "beta", "gamma"],
        "three items is three passes, each with its own value"
    );

    // And the frame reported where it had got to, each time.
    let progress: Vec<(u32, u32)> = events
        .iter()
        .filter_map(|e| match e {
            RunEvent::FrameState { at, of, .. } => Some((*at, *of)),
            _ => None,
        })
        .collect();
    assert_eq!(progress, [(0, 3), (1, 3), (2, 3), (3, 3)]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// `parallel` means at the same time, not one after another.
///
/// Each item sleeps; four items two at a time should take about two sleeps
/// rather than four. The margin is wide because a loaded machine is slow, but
/// the difference between two and four sleeps is not a margin.
#[test]
fn parallel_items_really_run_at_the_same_time() {
    let sleep = "0.4";
    let one_at_a_time = {
        let graph = looping(
            r#"["a","b","c","d"]"#,
            &format!("sleep {sleep}"),
            1,
            100,
            true,
        );
        let started = std::time::Instant::now();
        run(&graph);
        started.elapsed()
    };
    let two_at_a_time = {
        let graph = looping(
            r#"["a","b","c","d"]"#,
            &format!("sleep {sleep}"),
            2,
            100,
            true,
        );
        let started = std::time::Instant::now();
        run(&graph);
        started.elapsed()
    };

    assert!(
        one_at_a_time.as_secs_f64() > 1.5,
        "four sleeps of {sleep}s in sequence: {one_at_a_time:?}"
    );
    assert!(
        two_at_a_time.as_secs_f64() < one_at_a_time.as_secs_f64() * 0.75,
        "two at a time should be much faster than one: {two_at_a_time:?} vs {one_at_a_time:?}"
    );
}

/// `max` is a ceiling, so a folder holding ten thousand files does not become
/// ten thousand model calls because nobody looked.
#[test]
fn max_stops_the_loop_from_running_away() {
    let dir = scratch("max");
    let graph = looping(
        r#"["1","2","3","4","5","6","7","8"]"#,
        &format!("echo x >> {}/seen", dir.display()),
        1,
        3,
        true,
    );
    let (_, events, _) = run(&graph);
    let seen = std::fs::read_to_string(dir.join("seen")).unwrap();
    assert_eq!(seen.lines().count(), 3);

    let done = events
        .iter()
        .find_map(|e| match e {
            RunEvent::BlockDone { block, figure, .. } if block == "each" => figure.clone(),
            _ => None,
        })
        .unwrap();
    assert_eq!(done, "3 items");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Continue-on-error is the difference between losing one item and losing the
/// batch. The failures come out on the frame's `errors` port with the item
/// that caused each.
#[test]
fn continue_on_error_loses_the_item_and_not_the_batch() {
    let dir = scratch("keep-going");
    let items = format!(
        r#"["echo a >> {0}/seen","exit 3","echo c >> {0}/seen"]"#,
        dir.display()
    );
    let mut graph = looping(&items, "true", 1, 100, true);
    graph.wires.push(graph_format::Wire {
        id: "w2".into(),
        from: graph_format::Endpoint::new("each", "item"),
        to: graph_format::Endpoint::new("work", "text"),
    });

    let (summary, events, _) = run(&graph);
    let seen = std::fs::read_to_string(dir.join("seen")).unwrap_or_default();
    assert_eq!(
        seen.lines().collect::<Vec<_>>(),
        ["a", "c"],
        "the item between them failed and the rest still ran"
    );
    assert_eq!(summary.outcome, RunOutcome::Finished);

    let outputs = events
        .iter()
        .find_map(|e| match e {
            RunEvent::BlockDone { block, outputs, .. } if block == "each" => Some(outputs.clone()),
            _ => None,
        })
        .unwrap();
    let errors = outputs
        .iter()
        .find(|p| p.port == "errors")
        .expect("an errors port");
    let Value::Data(serde_json::Value::Array(rows)) = &errors.value else {
        panic!("errors is a list, not {:?}", errors.value);
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["item"], serde_json::json!("exit 3"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Without it, the first failure ends the loop.
#[test]
fn stopping_on_error_ends_the_loop_at_the_first_one() {
    let dir = scratch("stop-on-error");
    let items = format!(
        r#"["echo a >> {0}/seen","exit 3","echo c >> {0}/seen"]"#,
        dir.display()
    );
    let mut graph = looping(&items, "true", 1, 100, false);
    graph.wires.push(graph_format::Wire {
        id: "w2".into(),
        from: graph_format::Endpoint::new("each", "item"),
        to: graph_format::Endpoint::new("work", "text"),
    });

    let (summary, _, _) = run(&graph);
    let seen = std::fs::read_to_string(dir.join("seen")).unwrap_or_default();
    assert_eq!(
        seen.lines().collect::<Vec<_>>(),
        ["a"],
        "it stopped at the failure"
    );
    assert_eq!(summary.outcome, RunOutcome::Failed);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A block inside a frame that warns asks once for the whole loop.
///
/// A prompt that appears two hundred times is not a prompt, and answering it
/// two hundred times is not consent — it is attrition.
#[test]
fn a_warning_inside_a_loop_is_asked_once_for_all_the_items() {
    let dir = scratch("warning");
    let mut graph = looping(
        r#"["a","b","c","d","e"]"#,
        &format!("echo x >> {}/seen", dir.display()),
        1,
        100,
        true,
    );
    graph
        .blocks
        .iter_mut()
        .find(|b| b.id == "work")
        .unwrap()
        .settings
        .insert("warnBefore".into(), graph_format::Setting::Bool(true));

    let (_, _, asked) = run(&graph);
    assert_eq!(asked.len(), 1, "asked {} times", asked.len());
    assert!(asked[0].action.contains("5 items"), "{}", asked[0].action);
    // And having said yes once, all five ran.
    let seen = std::fs::read_to_string(dir.join("seen")).unwrap();
    assert_eq!(seen.lines().count(), 5);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A single value is one item. A folder source hands the frame one file at a
/// time, and a frame that refused to iterate a single value would make the
/// commonest live graph impossible to write.
#[test]
fn a_single_value_is_one_item() {
    let dir = scratch("single");
    let graph = looping(
        "just the one",
        &format!("echo x >> {}/seen", dir.display()),
        1,
        100,
        true,
    );
    let (summary, _, _) = run(&graph);
    assert_eq!(
        summary.outcome,
        RunOutcome::Finished,
        "{:?}",
        summary.errors
    );
    let seen = std::fs::read_to_string(dir.join("seen")).unwrap();
    assert_eq!(seen.lines().count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}
