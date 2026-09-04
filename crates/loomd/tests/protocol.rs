//! Driving a run through the protocol, the way the shell does.
//!
//! `run_triage` proves the engine runs a graph; this proves the *connection*
//! does — that events reach a reader while the run is still going, that a
//! warning really parks the run until it is answered, and that a reply written
//! from the read thread never lands in the middle of an event written from a
//! run thread.

use loomd::{Engine, Workspace};
use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc::{Receiver, channel};

/// A pipe the test writes requests into and reads lines out of, with the engine
/// serving in between — the same call `main` makes.
struct Wire {
    requests: std::os::unix::net::UnixStream,
    lines: Receiver<serde_json::Value>,
    /// Every line the engine has sent so far.
    ///
    /// Kept rather than consumed, because assertions do not arrive in the order
    /// the events do: a test that waits for `run.finished` and then looks for
    /// the `block.error` that preceded it should find it, not discover it was
    /// eaten on the way past.
    seen: Vec<serde_json::Value>,
    engine: Option<std::thread::JoinHandle<()>>,
}

impl Wire {
    fn open() -> Self {
        let (theirs, mine) = std::os::unix::net::UnixStream::pair().unwrap();
        let read = theirs.try_clone().unwrap();
        let engine = std::thread::spawn(move || {
            let ws =
                Workspace::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures")).unwrap();
            Engine::new(ws).serve(read, theirs);
        });

        let (tx, lines) = channel();
        let out = mine.try_clone().unwrap();
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line)
                    && tx.send(value).is_err()
                {
                    break;
                }
            }
        });

        Self {
            requests: mine,
            lines,
            seen: Vec::new(),
            engine: Some(engine),
        }
    }

    fn send(&mut self, id: u64, method: &str, params: serde_json::Value) {
        let mut request = serde_json::json!({ "id": id, "method": method });
        if !params.is_null() {
            request["params"] = params;
        }
        writeln!(self.requests, "{request}").unwrap();
        self.requests.flush().unwrap();
    }

    /// The next line matching a predicate, or a panic saying what was seen
    /// instead — a timeout that prints nothing is a bad way to fail.
    fn next(&mut self, what: &str, ok: impl Fn(&serde_json::Value) -> bool) -> serde_json::Value {
        if let Some(line) = self.seen.iter().find(|l| ok(l)) {
            return line.clone();
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while std::time::Instant::now() < deadline {
            let Ok(line) = self
                .lines
                .recv_timeout(std::time::Duration::from_millis(100))
            else {
                continue;
            };
            self.seen.push(line.clone());
            if ok(&line) {
                return line;
            }
        }
        panic!(
            "never saw {what}; saw {} lines: {:#?}",
            self.seen.len(),
            self.seen
        );
    }

    fn close(mut self) {
        // Shutting the write half down rather than dropping the stream: the
        // reader thread holds a clone of the same socket, so a drop would
        // leave the engine's end open and its read loop waiting for a line
        // that is never coming.
        let _ = self.requests.shutdown(std::net::Shutdown::Write);
        if let Some(engine) = self.engine.take() {
            let _ = engine.join();
        }
    }
}

fn triage() -> serde_json::Value {
    let graph = graph_format::load(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/graphs/customer-triage.loom"
    ))
    .unwrap();
    serde_json::to_value(graph).unwrap()
}

/// The whole shape in one test: start a run, get its id back, watch the
/// warning arrive, answer it, and see the run end.
///
/// It ends `failed` rather than `finished` because there is no Ollama on the
/// machine running this, and that is the honest outcome: the engine reports
/// that the model is not answering and the rest of the graph still completes.
/// The graph reaching its end *at all* through a socket is what is being
/// proven here — the model's own behaviour is `run_triage`'s job.
#[test]
fn a_run_is_driven_over_the_wire() {
    let mut wire = Wire::open();

    // The engine points at an Ollama that is not there, so the model fails
    // fast rather than the test waiting on a network timeout.
    let mut graph = triage();
    graph["blocks"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|b| b["kind"] == "llm")
        .unwrap()["settings"]["endpoint"] = serde_json::json!("http://127.0.0.1:1");

    wire.send(
        1,
        "run.start",
        serde_json::json!({ "path": "graphs/customer-triage.loom", "graph": graph }),
    );

    let started = wire.next("the reply to run.start", |l| l["id"] == 1);
    assert_eq!(started["result"], "running", "{started}");
    let run = started["data"]["run"].as_str().unwrap().to_owned();
    assert!(run.starts_with("run-"), "{run}");

    // Events name the run, so a shell with two graphs open knows which canvas
    // to draw on.
    let begin = wire.next("run.started", |l| l["event"] == "run.started");
    assert_eq!(begin["data"]["run"], run);
    assert_eq!(begin["data"]["graph"], "customer-triage");
    assert_eq!(
        begin["data"]["order"],
        serde_json::json!(["input", "llm", "report"])
    );

    let finished = wire.next("run.finished", |l| l["event"] == "run.finished");
    assert_eq!(finished["data"]["outcome"], "failed");

    // And it said why, in a sentence naming what to do about it.
    let complaint = wire.next("the console line about ollama", |l| {
        l["event"] == "block.error"
            && l["data"]["message"]
                .as_str()
                .is_some_and(|m| m.contains("ollama serve"))
    });
    assert_eq!(complaint["data"]["block"], "llm");

    wire.close();
}

/// The warning parks the run. Nothing downstream of it happens until a person
/// answers, which is what makes SPEC §12.1 a pause rather than a notification.
#[test]
fn a_warning_stops_the_run_until_it_is_answered() {
    let mut wire = Wire::open();

    // A graph of one Terminal that warns: the smallest thing that asks.
    let mut graph = triage();
    graph["blocks"] = serde_json::json!([{
        "id": "terminal", "kind": "terminal", "position": [0, 0], "view": "summary",
        "settings": { "command": "echo ran", "warnBefore": true },
        "ports": [], "disabled": false, "breakpoint": false
    }]);
    graph["wires"] = serde_json::json!([]);

    wire.send(
        1,
        "run.start",
        serde_json::json!({ "path": "graphs/customer-triage.loom", "graph": graph }),
    );
    wire.next("the reply", |l| l["id"] == 1);

    let warning = wire.next("run.warning", |l| l["event"] == "run.warning");
    assert_eq!(warning["data"]["block"], "terminal");
    assert!(
        warning["data"]["action"]
            .as_str()
            .unwrap()
            .contains("echo ran"),
        "{warning}"
    );

    // While it waits, the engine is still answering questions. A run that
    // blocked the connection would make Stop unreachable.
    wire.send(2, "engine.status", serde_json::Value::Null);
    let status = wire.next("a reply while parked", |l| l["id"] == 2);
    assert_eq!(status["result"], "engineStatus");

    let id = warning["data"]["id"].as_str().unwrap();
    wire.send(
        3,
        "run.continue",
        serde_json::json!({ "warning": id, "decision": "continue" }),
    );
    let ack = wire.next("the reply to run.continue", |l| l["id"] == 3);
    assert_eq!(ack["data"]["ok"], true);

    // Only now does the command run.
    let done = wire.next("run.finished", |l| l["event"] == "run.finished");
    assert_eq!(done["data"]["outcome"], "finished");
    wire.close();
}

/// Answering with `stop` stops the run and the command never runs.
#[test]
fn stopping_at_a_warning_never_runs_the_command() {
    let mut wire = Wire::open();
    let marker = std::env::temp_dir().join(format!("cyberloom-{}.marker", std::process::id()));
    let _ = std::fs::remove_file(&marker);

    let mut graph = triage();
    graph["blocks"] = serde_json::json!([{
        "id": "terminal", "kind": "terminal", "position": [0, 0], "view": "summary",
        "settings": { "command": format!("touch {}", marker.display()), "warnBefore": true },
        "ports": [], "disabled": false, "breakpoint": false
    }]);
    graph["wires"] = serde_json::json!([]);

    wire.send(
        1,
        "run.start",
        serde_json::json!({ "path": "graphs/customer-triage.loom", "graph": graph }),
    );
    let warning = wire.next("run.warning", |l| l["event"] == "run.warning");
    wire.send(
        2,
        "run.continue",
        serde_json::json!({
            "warning": warning["data"]["id"].as_str().unwrap(),
            "decision": "stop"
        }),
    );

    let done = wire.next("run.finished", |l| l["event"] == "run.finished");
    assert_eq!(done["data"]["outcome"], "stopped");
    assert!(
        !marker.exists(),
        "the command ran even though the user said stop"
    );
    wire.close();
}

/// Every line on the socket is one whole JSON object. A run writing events
/// while the read thread writes a reply must never produce a half of either.
#[test]
fn replies_and_events_never_interleave() {
    let mut wire = Wire::open();
    let mut graph = triage();
    // Twenty blocks all talking at once, to give two writers a chance to
    // collide if they were ever going to.
    graph["blocks"] = serde_json::Value::Array(
        (0..20)
            .map(|i| {
                serde_json::json!({
                    "id": format!("t{i}"), "kind": "terminal",
                    "position": [0, 0], "view": "summary",
                    "settings": { "command": "echo hello" },
                    "ports": [], "disabled": false, "breakpoint": false
                })
            })
            .collect(),
    );
    graph["wires"] = serde_json::json!([]);

    wire.send(
        1,
        "run.start",
        serde_json::json!({ "path": "graphs/customer-triage.loom", "graph": graph }),
    );
    for id in 2..12 {
        wire.send(id, "engine.status", serde_json::Value::Null);
    }

    // Every reply arrives, and the reader parsed every line it saw: a torn
    // line would have failed to parse and never reached the channel at all.
    for id in 2..12 {
        let reply = wire.next(&format!("reply {id}"), |l| l["id"] == id);
        assert_eq!(reply["result"], "engineStatus");
    }
    let done = wire.next("run.finished", |l| l["event"] == "run.finished");
    assert_eq!(done["data"]["outcome"], "finished");
    wire.close();
}
