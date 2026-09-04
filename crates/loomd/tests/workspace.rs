//! Local only, and what happens when it is not (SPEC §15.4, §12.2).
//!
//! > A per-graph Local only switch, on by default. Turning it off allows remote
//! > model providers; the first send of a run to any remote service warns.
//!
//! The switch is the user's own, so a graph with it on does not send — that is
//! the setting doing what it says rather than the application overruling
//! anybody (§12.1). With it off, the send warns once per run: a conversation is
//! one send as far as a person is concerned, and asking eight times through a
//! tool loop is how a prompt becomes something people click past.

use loomd::run::event::RunEvent;
use loomd::run::model::{ChatRequest, ChatTurn, ModelError, ModelProvider, Sink, Usage};
use loomd::run::runner::{Bench, Decision, Runner, Warning};
use std::sync::Arc;

fn fixtures() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures"))
}

/// A model that is somewhere else. The only thing that matters about it here is
/// that `local()` is false.
struct Elsewhere {
    turns: std::sync::Mutex<std::collections::VecDeque<ChatTurn>>,
    pub sends: std::sync::atomic::AtomicUsize,
}

impl Elsewhere {
    fn new(turns: impl IntoIterator<Item = ChatTurn>) -> Self {
        Self {
            turns: std::sync::Mutex::new(turns.into_iter().collect()),
            sends: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

impl ModelProvider for Elsewhere {
    fn name(&self) -> &str {
        "a remote service"
    }

    fn local(&self) -> bool {
        false
    }

    fn chat(&self, _request: &ChatRequest, _sink: Sink<'_>) -> Result<ChatTurn, ModelError> {
        self.sends
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.turns
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| ModelError::Refused("the script has no more turns".into()))
    }
}

fn says(text: &str) -> ChatTurn {
    ChatTurn {
        text: text.to_owned(),
        tool_calls: Vec::new(),
        usage: Usage::default(),
    }
}

struct Ran {
    warnings: Vec<Warning>,
    console: Vec<String>,
    sends: usize,
}

fn run(local_only: bool, answer: Decision) -> Ran {
    let mut graph = graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap();
    graph.local_only = local_only;
    // Two turns, so a graph that sends more than once would show it.
    let provider = Elsewhere::new([says("first"), says("second")]);
    let root = fixtures();
    let mut console = Vec::new();
    let mut warnings = Vec::new();
    Runner {
        graph: &graph,
        root: &root,
        provider: &provider,
        run: "remote".into(),
        cancel: Default::default(),
        scratch: Arc::new(loomd::run::sense::Scratch::open("workspace-test").unwrap()),
        eye: Arc::new(loomd::run::perceive::Scripted::default()),
        vault: Arc::new(loomd::run::memory::Vault::new(&root)),
        bench: Arc::new(Bench::scripted()),
    }
    .execute(
        &mut |e| {
            if let RunEvent::Console { message, .. } = &e {
                console.push(message.clone());
            }
        },
        &mut |w| {
            warnings.push(w.clone());
            answer
        },
    );
    Ran {
        warnings,
        console,
        sends: provider.sends.load(std::sync::atomic::Ordering::Relaxed),
    }
}

/// On by default, and it means what it says.
#[test]
fn local_only_sends_nothing_and_says_where_the_switch_is() {
    let ran = run(true, Decision::Continue);
    assert_eq!(ran.sends, 0, "it sent anyway");
    assert!(
        ran.warnings.is_empty(),
        "it asked about something it would not do"
    );
    let complaint = ran
        .console
        .iter()
        .find(|line| line.contains("Local only"))
        .unwrap_or_else(|| panic!("it never said why: {:?}", ran.console));
    assert!(
        complaint.contains("graph panel"),
        "it should point at the switch: {complaint}"
    );
}

/// Off: the send warns, once, and then goes.
#[test]
fn with_local_only_off_the_first_send_warns_and_then_goes() {
    let ran = run(false, Decision::Continue);
    assert_eq!(ran.warnings.len(), 1, "warnings: {:?}", ran.warnings);
    assert!(
        ran.warnings[0].action.contains("a remote service"),
        "{:?}",
        ran.warnings[0]
    );
    assert!(
        ran.warnings[0].reason.contains("leaves here"),
        "the warning should say what it costs: {:?}",
        ran.warnings[0]
    );
    assert!(ran.sends >= 1, "it warned and then did not send");
}

/// §12.1: the answer is the user's, and no is an answer.
#[test]
fn saying_no_sends_nothing() {
    let ran = run(false, Decision::Stop);
    assert_eq!(ran.warnings.len(), 1);
    assert_eq!(ran.sends, 0, "it sent after being told not to");
}
