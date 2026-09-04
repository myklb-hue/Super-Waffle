//! The customer-triage example, run end to end (SPEC §13.1).
//!
//! > Run once. The LLM calls `terminal.run`, the terminal fails with exit 101,
//! > the model reads the linker error and answers.
//!
//! That sentence is the test. The model's side of it is scripted, because a
//! test that needs a language model installed and in a particular mood is not a
//! test — see `run::model`. Everything on the engine's side is real: the plan,
//! the tool loop, the shell that runs, the exit code that comes back, and every
//! event the shell would draw.

use loomd::run::event::{BlockState, RunEvent, RunOutcome};
use loomd::run::model::{ChatTurn, Scripted, ToolCall, Usage};
use loomd::run::runner::{Decision, Runner, Warning};
use loomd::run::value::Value;

fn fixtures() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures"))
}

fn triage() -> graph_format::Graph {
    graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap()
}

/// The two turns the example describes: call the terminal, then answer.
fn script() -> Scripted {
    Scripted::new([
        ChatTurn {
            text: String::new(),
            tool_calls: vec![ToolCall {
                name: "terminal_run".into(),
                // The command that fails the way §13.1 needs it to.
                arguments: serde_json::json!({ "command": "echo 'error: linking with `cc` failed' >&2; exit 101" }),
            }],
            usage: Usage::default(),
        },
        ChatTurn {
            text: "The build fails at the link step: the linker returned 101 and \
                   the error names `cc`. A missing system library is the usual cause."
                .into(),
            tool_calls: vec![],
            usage: Usage {
                tokens_in: 210,
                tokens_out: 34,
                rate: 41.0,
            },
        },
    ])
}

struct Recorded {
    events: Vec<RunEvent>,
    asked: Vec<Warning>,
}

fn run_it(decide: Decision) -> (loomd::run::runner::Summary, Recorded) {
    let graph = triage();
    let provider = script();
    let root = fixtures();
    let runner = Runner {
        graph: &graph,
        root: &root,
        provider: &provider,
        run: "r1".into(),
        cancel: Default::default(),
        scratch: std::sync::Arc::new(loomd::run::sense::Scratch::open("t").unwrap()),
        eye: std::sync::Arc::new(loomd::run::perceive::Scripted::default()),
    };
    let mut events = Vec::new();
    let mut asked = Vec::new();
    let summary = runner.execute(&mut |e| events.push(e), &mut |w| {
        asked.push(w.clone());
        decide
    });
    (summary, Recorded { events, asked })
}

#[test]
fn the_model_calls_the_terminal_reads_the_failure_and_answers() {
    let (summary, log) = run_it(Decision::Continue);

    assert_eq!(
        summary.outcome,
        RunOutcome::Finished,
        "{:?}",
        summary.errors
    );

    // The tool call happened, and it was named the way the spec names it.
    let call = log
        .events
        .iter()
        .find_map(|e| match e {
            RunEvent::ToolCall { name, callee, .. } => Some((name.clone(), callee.clone())),
            _ => None,
        })
        .expect("the model called a tool");
    assert_eq!(call, ("terminal.run".to_owned(), "terminal".to_owned()));

    // The terminal really ran, and really failed with 101.
    let result = log
        .events
        .iter()
        .find_map(|e| match e {
            RunEvent::ToolResult { result, ok, .. } => Some((result.clone(), *ok)),
            _ => None,
        })
        .expect("the tool returned");
    assert!(!result.1, "exit 101 is not a success");
    assert!(result.0.starts_with("exit 101"), "{}", result.0);
    assert!(result.0.contains("linking with"), "{}", result.0);

    // And the answer reached the Output block, under the name it was given.
    assert_eq!(summary.results.len(), 1);
    assert_eq!(summary.results[0].port, "report");
    let Value::Text(answer) = &summary.results[0].value else {
        panic!("the report is text");
    };
    assert!(answer.contains("link step"), "{answer}");
}

/// The Terminal in this graph has `warnBefore: true`, so a shell command does
/// not run until a person says so (SPEC §12.1, §12.2).
#[test]
fn the_terminal_asks_before_it_runs() {
    let (_, log) = run_it(Decision::Continue);
    assert_eq!(log.asked.len(), 1);
    let warning = &log.asked[0];
    assert_eq!(warning.block, "terminal");
    assert!(warning.action.contains("exit 101"), "{}", warning.action);
    assert!(warning.reason.contains("dangerous"), "{}", warning.reason);
    assert!(warning.remember, "the prompt offers to stop asking");
}

/// Stopping at the warning stops the run, and nothing downstream of the model
/// pretends to have a value.
#[test]
fn stopping_at_the_warning_stops_the_run() {
    let (summary, log) = run_it(Decision::Stop);
    assert_eq!(summary.outcome, RunOutcome::Stopped);
    assert!(summary.results.is_empty(), "{:?}", summary.results);
    assert!(
        log.events.iter().any(|e| matches!(
            e,
            RunEvent::BlockError { block, .. } if block == "llm"
        )),
        "the model reports that it could not finish"
    );
}

/// What the canvas draws. The three capabilities stand ready rather than
/// running, which is the distinction the plan exists for.
#[test]
fn the_canvas_is_told_what_every_block_is_doing() {
    let (_, log) = run_it(Decision::Continue);
    let mut last: std::collections::BTreeMap<String, BlockState> = Default::default();
    let mut ever: std::collections::BTreeMap<String, Vec<BlockState>> = Default::default();
    for event in &log.events {
        if let RunEvent::BlockState { block, state, .. } = event {
            last.insert(block.clone(), *state);
            ever.entry(block.clone()).or_default().push(*state);
        }
    }

    assert_eq!(last["input"], BlockState::Done);
    assert_eq!(last["llm"], BlockState::Done);
    assert_eq!(last["report"], BlockState::Done);
    // Python was never called, so it stands ready and never ran.
    assert_eq!(last["python"], BlockState::Ready);
    assert!(!ever["python"].contains(&BlockState::Running));
    // The terminal was called, so it ran — and went back to ready afterwards,
    // because it may be called again.
    assert!(ever["terminal"].contains(&BlockState::Running));
    assert_eq!(last["terminal"], BlockState::Ready);
}

/// Every wire that carried something says so, which is what animates it.
#[test]
fn the_wires_that_carried_a_value_light_up() {
    let (_, log) = run_it(Decision::Continue);
    let lit: Vec<&str> = log
        .events
        .iter()
        .filter_map(|e| match e {
            RunEvent::WireActive { wire, .. } => Some(wire.as_str()),
            _ => None,
        })
        .collect();
    // w1 is input → llm, w5 is llm → report. The handle wires (w2, w3, w4)
    // never carry a value, because a handle is not a flow (SPEC §4.3).
    assert_eq!(lit, ["w1", "w5"]);
}

/// The model was offered the two runtimes, by the names SPEC §6.3 gives them,
/// and never the Toolbox between them.
#[test]
fn the_model_is_offered_the_runtimes_not_the_box() {
    let (_, _log) = run_it(Decision::Continue);
    let graph = triage();
    let provider = script();
    let root = fixtures();
    Runner {
        graph: &graph,
        root: &root,
        provider: &provider,
        run: "r2".into(),
        cancel: Default::default(),
        scratch: std::sync::Arc::new(loomd::run::sense::Scratch::open("t").unwrap()),
        eye: std::sync::Arc::new(loomd::run::perceive::Scripted::default()),
    }
    .execute(&mut |_| {}, &mut |_| Decision::Continue);

    let seen = provider.seen.lock().unwrap();
    let offered: Vec<String> = seen[0].tools.iter().map(|t| t.name.clone()).collect();
    assert_eq!(offered, ["terminal_run", "python_exec"]);
    assert!(seen[0].messages.iter().any(|m| matches!(
        m,
        loomd::run::model::Message::User { content } if content.contains("triage ticket")
    )));
}

/// The model's answer streams as it is written rather than appearing whole
/// (SPEC §5.3).
#[test]
fn the_answer_arrives_in_pieces() {
    let (_, log) = run_it(Decision::Continue);
    let chunks: Vec<&str> = log
        .events
        .iter()
        .filter_map(|e| match e {
            RunEvent::BlockOutput { chunk, block, .. } if block == "llm" => Some(chunk.as_str()),
            _ => None,
        })
        .collect();
    assert!(chunks.len() > 5, "streamed in {} pieces", chunks.len());
    assert!(chunks.concat().contains("link step"));
}

/// A local model costs nothing, and the Run panel says so rather than showing a
/// zero (SPEC §8.5).
#[test]
fn usage_says_the_model_is_local() {
    let (_, log) = run_it(Decision::Continue);
    let usage = log
        .events
        .iter()
        .rev()
        .find_map(|e| match e {
            RunEvent::Usage {
                tokens_out,
                local,
                rate,
                ..
            } => Some((*tokens_out, *local, *rate)),
            _ => None,
        })
        .expect("usage is reported");
    assert_eq!(usage.0, 34);
    assert!(usage.1, "a scripted provider runs here");
    assert!((usage.2 - 41.0).abs() < 0.01);
}
