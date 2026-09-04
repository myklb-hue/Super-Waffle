//! What the graph remembers (SPEC §6.5, §9.2, §13.3).
//!
//! > Two stores, one handle, consolidation every ten minutes, the orchestrator
//! > writing one line per episode.
//!
//! The acceptance for this slice is that the assistant remembers a name across
//! runs, so that is the test at the bottom: one run is told a name and stores
//! it, a second run — a new engine, a new working memory, nothing in common but
//! the file on disk — is asked and answers.

use loomd::run::event::RunEvent;
use loomd::run::memory::{Store, Vault};
use loomd::run::model::{ChatTurn, Scripted, Usage};
use loomd::run::runner::{Decision, Runner, Warning};
use std::sync::Arc;

fn workspace(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cyberloom-memory-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
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

/// Keyboard → LLM (with a memory hub over working and long-term stores).
///
/// The shape of Figure 9's memory chain, with the parts that are not about
/// memory left out.
fn remembering(db: &std::path::Path, prompt: &str, items: &str) -> graph_format::Graph {
    let mut graph = graph_format::load(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/graphs/customer-triage.loom"
    ))
    .unwrap();
    graph.run_mode = graph_format::RunMode::Once;
    graph.blocks = vec![
        block("ask", "keyboard", &[("placeholder", prompt)]),
        block("working", "working-memory", &[("items", items)]),
        block(
            "longterm",
            "long-term-memory",
            &[("path", &db.display().to_string())],
        ),
        block(
            "hub",
            "memory-hub",
            &[("summarise", "false"), ("maxRecalled", "12")],
        ),
        block("orchestrator", "llm", &[("role", "orchestrator")]),
        block("out", "output", &[]),
    ];
    graph.wires = vec![
        wire("w1", ("ask", "text"), ("orchestrator", "prompt")),
        wire("w2", ("working", "memory"), ("hub", "memory")),
        wire("w3", ("longterm", "memory"), ("hub", "memory")),
        wire("w4", ("hub", "memory"), ("orchestrator", "memory")),
        wire("w5", ("orchestrator", "text"), ("out", "value")),
    ];
    graph.frames.clear();
    graph
}

struct Ran {
    events: Vec<RunEvent>,
    provider: Scripted,
    warnings: Vec<Warning>,
}

impl Ran {
    fn console(&self) -> Vec<String> {
        self.events
            .iter()
            .filter_map(|e| match e {
                RunEvent::Console { message, .. } => Some(message.clone()),
                _ => None,
            })
            .collect()
    }

    /// What every tool call answered with, which is where memory says what it
    /// did: the console narrates a run, and one line per remembered fact would
    /// bury everything else in a graph that remembers often.
    fn results(&self) -> Vec<String> {
        self.events
            .iter()
            .filter_map(|e| match e {
                RunEvent::ToolResult { result, .. } => Some(result.clone()),
                _ => None,
            })
            .collect()
    }

    /// Everything the model was told, flattened — enough to assert that a
    /// recalled memory really did reach it.
    fn told(&self) -> String {
        self.provider
            .seen
            .lock()
            .unwrap()
            .iter()
            .flat_map(|r| r.messages.iter().map(|m| format!("{m:?}")))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn run(graph: &graph_format::Graph, root: &std::path::Path, turns: Vec<ChatTurn>) -> Ran {
    run_answering(graph, root, turns, Decision::Continue)
}

fn run_answering(
    graph: &graph_format::Graph,
    root: &std::path::Path,
    turns: Vec<ChatTurn>,
    answer: Decision,
) -> Ran {
    let provider = Scripted::new(turns);
    let mut events = Vec::new();
    let mut warnings = Vec::new();
    Runner {
        graph,
        root,
        provider: &provider,
        run: "memory".into(),
        cancel: Default::default(),
        scratch: Arc::new(loomd::run::sense::Scratch::open("memory-test").unwrap()),
        eye: Arc::new(loomd::run::perceive::Scripted::default()),
        vault: Arc::new(Vault::new(root)),
        bench: Arc::new(loomd::run::runner::Bench::scripted()),
    }
    .execute(&mut |e| events.push(e), &mut |w| {
        warnings.push(w.clone());
        answer
    });
    Ran {
        events,
        provider,
        warnings,
    }
}

fn calls(name: &str, arguments: serde_json::Value) -> ChatTurn {
    Scripted::calls(name, arguments)
}

fn says(text: &str) -> ChatTurn {
    Scripted::says(text)
}

/// §9.2: a model with a memory handle gets `remember()` and `forget()` as
/// tools — and, deliberately, no `recall`.
#[test]
fn a_memory_handle_is_two_tools_and_not_three() {
    let root = workspace("tools");
    let graph = remembering(&root.join("memory.db"), "hello", "8");
    let ran = run(&graph, &root, vec![says("hello to you too")]);

    let seen = ran.provider.seen.lock().unwrap();
    let offered: Vec<&str> = seen[0].tools.iter().map(|t| t.name.as_str()).collect();
    assert!(
        offered.iter().any(|n| n.ends_with("remember")),
        "no remember in {offered:?}"
    );
    assert!(
        offered.iter().any(|n| n.ends_with("forget")),
        "no forget in {offered:?}"
    );
    assert!(
        !offered.iter().any(|n| n.contains("recall")),
        "recall is not a tool: {offered:?}"
    );
    // The hub is what the model calls, not the stores behind it. That is the
    // opposite of a Toolbox and it is the point of §9.2.
    assert!(
        offered.iter().all(|n| n.starts_with("hub")),
        "the stores leaked into the tool list: {offered:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn what_the_model_remembers_comes_back_in_the_next_run() {
    let root = workspace("across");
    let db = root.join("memory.db");

    // Run one: it is told a name and keeps it.
    let first = remembering(&db, "my name is Mykl", "8");
    let told = run(
        &first,
        &root,
        vec![
            calls(
                "hub.remember",
                serde_json::json!({ "text": "the user's name is Mykl", "kind": "person" }),
            ),
            says("nice to meet you, Mykl"),
        ],
    );
    assert!(
        told.results().iter().any(|r| r.contains("remembered")),
        "nothing was remembered: {:?}",
        told.results()
    );

    // Run two: a new engine, a new working memory, nothing carried over but
    // the file. Recall happens before the model says a word.
    let second = remembering(&db, "what is my name?", "8");
    let asked = run(&second, &root, vec![says("your name is Mykl")]);
    assert!(
        asked.told().contains("the user's name is Mykl"),
        "the model was never told what it knew:\n{}",
        asked.told()
    );
    assert!(
        asked.console().iter().any(|l| l.contains("recalled")),
        "the console never said it recalled anything: {:?}",
        asked.console()
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// §9.2: consolidation carries what working memory has learned into the
/// long-term store — and carries each thing once.
#[test]
fn what_working_memory_learns_reaches_the_long_term_store() {
    let root = workspace("consolidate");
    let db = root.join("memory.db");
    let graph = remembering(&db, "hello", "8");
    let ran = run(
        &graph,
        &root,
        vec![
            calls(
                "hub.remember",
                serde_json::json!({ "text": "the door was open" }),
            ),
            calls(
                "hub.remember",
                serde_json::json!({ "text": "the cat came in" }),
            ),
            says("noted"),
        ],
    );
    assert!(
        ran.console().iter().any(|l| l.contains("consolidated")),
        "nothing was consolidated: {:?}",
        ran.console()
    );

    // And it is really on disk, in a store this test opens for itself.
    let store = loomd::run::memory::LongTerm::open(&db).unwrap();
    let mut kept: Vec<String> = store
        .recall(None, 8, 0.0)
        .unwrap()
        .into_iter()
        .map(|e| e.text)
        .collect();
    kept.sort();
    assert_eq!(kept, ["the cat came in", "the door was open"]);
    let _ = std::fs::remove_dir_all(&root);
}

/// §12.2: deleting a person from long-term memory warrants a warning. §12.1:
/// the warning warns, and the user decides.
#[test]
fn forgetting_asks_first_and_the_answer_is_obeyed() {
    let root = workspace("forget");
    let db = root.join("memory.db");
    let graph = remembering(&db, "forget Sam", "8");

    let refused = run_answering(
        &graph,
        &root,
        vec![
            calls("hub.forget", serde_json::json!({ "what": "Sam" })),
            says("I have left it alone"),
        ],
        Decision::Stop,
    );
    assert_eq!(refused.warnings.len(), 1, "it did not ask");
    assert!(
        refused.warnings[0].action.contains("Sam"),
        "the warning did not say what would go: {:?}",
        refused.warnings[0]
    );

    // And when the answer is yes, it goes.
    let store = loomd::run::memory::LongTerm::open(&db).unwrap();
    store
        .remember("Sam is the partner", "person", None)
        .unwrap();
    store
        .remember("saw Sam at the door", "episode", None)
        .unwrap();
    drop(store);

    let agreed = run(
        &graph,
        &root,
        vec![
            calls("hub.forget", serde_json::json!({ "what": "Sam" })),
            says("forgotten"),
        ],
    );
    assert!(
        agreed.results().iter().any(|r| r.contains("forgot 2")),
        "results: {:?}",
        agreed.results()
    );
    let store = loomd::run::memory::LongTerm::open(&db).unwrap();
    assert_eq!(store.len(), 0, "the sightings outlived the person");
    let _ = std::fs::remove_dir_all(&root);
}

/// A store wired straight into a model, with no hub between them, is a hub of
/// one — the same arrangement §9.1 allows for tools.
#[test]
fn a_store_wired_straight_into_a_model_still_works() {
    let root = workspace("direct");
    let mut graph = remembering(&root.join("memory.db"), "hello", "8");
    graph.blocks.retain(|b| b.id != "hub" && b.id != "longterm");
    graph.wires = vec![
        wire("w1", ("ask", "text"), ("orchestrator", "prompt")),
        wire("w2", ("working", "memory"), ("orchestrator", "memory")),
        wire("w5", ("orchestrator", "text"), ("out", "value")),
    ];
    let ran = run(
        &graph,
        &root,
        vec![
            calls(
                "working.remember",
                serde_json::json!({ "text": "kept anyway" }),
            ),
            says("done"),
        ],
    );
    assert!(
        ran.results().iter().any(|r| r.contains("remembered")),
        "{:?}",
        ran.results()
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A turn that costs nothing, so a script reads as the conversation it is.
#[allow(dead_code)]
fn quiet() -> Usage {
    Usage::default()
}
