//! One connection, and the runs happening on it.
//!
//! Until this slice the protocol was strictly a question and an answer, on one
//! thread: read a line, handle it, write a line. A run cannot work that way. It
//! takes seconds or minutes, it has things to say the whole time, and the shell
//! has to stay able to ask it to stop — so the connection has to carry traffic
//! in both directions at once.
//!
//! The shape is deliberately the smallest one that does:
//!
//! - **one writer thread**, draining a channel of already-serialised lines.
//!   Everything that wants to say something sends a line rather than touching
//!   the socket, so two threads can never interleave halfway through a JSON
//!   object. It also means the read loop never blocks on a slow reader.
//! - **one thread per run.** A run is a long call into `runner`, which is
//!   synchronous by design; giving it a thread costs nothing and keeps the
//!   runtime free of an async runtime the rest of the engine does not need.
//! - **a rendezvous per warning.** The run thread parks on a channel and the
//!   read loop hands it the answer, which is what makes "warn, never block"
//!   (SPEC §12.1) a real pause rather than a notification the run ignores.

use crate::rpc::{Reply, ReplyEnvelope, RpcError};
use crate::run::event::RunEvent;
use crate::run::live::Live;
use crate::run::model::ModelProvider;
use crate::run::ollama::Ollama;
use crate::run::runner::{Decision, Runner, Warning};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

/// A line on its way out. Replies and events share one queue because they share
/// one socket, and the queue is what keeps them whole.
pub enum Outgoing {
    Line(String),
    /// The connection is finished; the writer stops.
    Done,
}

/// What the session knows about a run in flight.
struct InFlight {
    cancel: Arc<AtomicBool>,
    /// Set while a live graph is holding: events keep queueing and nothing
    /// runs until it clears (SPEC §8.1).
    paused: Arc<AtomicBool>,
    /// Warnings this run is parked on, by the id the shell answers with.
    pending: HashMap<String, Sender<Decision>>,
}

/// Everything one connection owns.
pub struct Session {
    out: Sender<Outgoing>,
    runs: Mutex<HashMap<String, InFlight>>,
    next: AtomicU64,
}

impl Session {
    pub fn new(out: Sender<Outgoing>) -> Self {
        Self {
            out,
            runs: Mutex::new(HashMap::new()),
            next: AtomicU64::new(1),
        }
    }

    pub fn send_reply(&self, id: u64, reply: Reply) {
        self.send(&ReplyEnvelope { id, reply });
    }

    pub fn send_event(&self, event: &RunEvent) {
        self.send(event);
    }

    fn send(&self, value: &impl serde::Serialize) {
        match serde_json::to_string(value) {
            // A send that fails means the writer is gone, which means the
            // connection is gone. There is nobody left to tell.
            Ok(line) => {
                let _ = self.out.send(Outgoing::Line(line));
            }
            Err(e) => {
                let _ = self.out.send(Outgoing::Line(
                    serde_json::json!({
                        "event": "console",
                        "data": { "run": "", "source": null, "level": "error",
                                  "message": format!("could not serialise an event: {e}") }
                    })
                    .to_string(),
                ));
            }
        }
    }

    fn fresh_run_id(&self) -> String {
        format!("run-{}", self.next.fetch_add(1, Ordering::Relaxed))
    }

    /// Start a run, on its own thread. Returns the id the shell will see on
    /// every event.
    pub fn start_run(
        self: &Arc<Self>,
        graph: graph_format::Graph,
        root: std::path::PathBuf,
    ) -> Result<String, RpcError> {
        let provider = provider_for(&graph)?;
        let run = self.fresh_run_id();
        let cancel = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        self.runs.lock().unwrap().insert(
            run.clone(),
            InFlight {
                cancel: cancel.clone(),
                paused: paused.clone(),
                pending: HashMap::new(),
            },
        );

        let session = Arc::clone(self);
        let id = run.clone();
        std::thread::Builder::new()
            .name(format!("loomd-{run}"))
            .spawn(move || {
                let emit = &mut |event: crate::run::event::RunEvent| session.send_event(&event);
                let ask = &mut |warning: &Warning| session.ask(&id, warning);
                // The graph says which it is. Once runs top to bottom and
                // stops; Live and Schedule arm the sources and never finish on
                // their own (SPEC §8.1).
                // One scratch folder for the whole run: frames and audio live
                // in it and go away with it (SPEC §12.3).
                let scratch = match crate::run::sense::Scratch::open(&id) {
                    Ok(s) => Arc::new(s),
                    Err(e) => {
                        session.send_event(&crate::run::event::RunEvent::Console {
                            run: id.clone(),
                            source: None,
                            level: crate::run::event::Level::Error,
                            message: format!("could not make a scratch folder: {e}"),
                        });
                        session.runs.lock().unwrap().remove(&id);
                        return;
                    }
                };
                // One vault for the whole run, for the same reason as the
                // scratch: working memory is windowed, and a window means
                // nothing if the store is rebuilt for every event.
                let vault = Arc::new(crate::run::memory::Vault::new(root.clone()));
                let eye: Arc<dyn crate::run::perceive::Perception> =
                    Arc::new(crate::run::perceive::Local::new(
                        models_folder(),
                        std::path::PathBuf::from("python3"),
                    ));

                match graph.run_mode {
                    graph_format::RunMode::Once => {
                        Runner {
                            graph: &graph,
                            root: &root,
                            provider: provider.as_ref(),
                            run: id.clone(),
                            cancel,
                            scratch,
                            eye,
                            vault,
                        }
                        .execute(emit, ask);
                    }
                    _ => {
                        Live {
                            graph: &graph,
                            root: &root,
                            provider: provider.as_ref(),
                            run: id.clone(),
                            cancel,
                            paused,
                            scratch,
                            eye,
                            vault,
                        }
                        .execute(emit, ask);
                    }
                }
                // A finished run is forgotten, so a `run.stop` for it is a
                // no-op rather than an error about a run that already ended.
                session.runs.lock().unwrap().remove(&id);
            })
            .map_err(|e| RpcError::new("run", format!("could not start a run: {e}")))?;
        Ok(run)
    }

    /// Ask the shell about a warning and park until it answers.
    ///
    /// A disconnected shell answers `Stop`: the run is waiting on a person who
    /// is no longer there, and the alternative is a shell command that runs
    /// because nobody was around to be asked.
    fn ask(&self, run: &str, warning: &Warning) -> Decision {
        let id = format!("{run}-w{}", self.next.fetch_add(1, Ordering::Relaxed));
        let (tx, rx) = channel();
        {
            let mut runs = self.runs.lock().unwrap();
            let Some(live) = runs.get_mut(run) else {
                return Decision::Stop;
            };
            live.pending.insert(id.clone(), tx);
        }
        self.send_event(&RunEvent::Warning {
            run: run.to_owned(),
            id: id.clone(),
            block: warning.block.clone(),
            action: warning.action.clone(),
            reason: warning.reason.clone(),
            remember: warning.remember,
        });
        let answer = rx.recv().unwrap_or(Decision::Stop);
        if let Some(live) = self.runs.lock().unwrap().get_mut(run) {
            live.pending.remove(&id);
        }
        answer
    }

    /// Answer a warning. False when there is no warning by that id, which is
    /// what a second click on Continue looks like.
    pub fn answer(&self, warning: &str, decision: Decision) -> bool {
        let mut runs = self.runs.lock().unwrap();
        for live in runs.values_mut() {
            if let Some(tx) = live.pending.remove(warning) {
                return tx.send(decision).is_ok();
            }
        }
        false
    }

    /// Stop a run, or every run when given no id.
    pub fn stop(&self, run: Option<&str>) -> usize {
        let mut runs = self.runs.lock().unwrap();
        let mut stopped = 0;
        for (id, live) in runs.iter_mut() {
            if run.is_some_and(|wanted| wanted != id) {
                continue;
            }
            live.cancel.store(true, Ordering::Relaxed);
            // A run parked on a warning would never see the flag, because it is
            // not running. Answering its warnings is what wakes it up.
            for (_, tx) in live.pending.drain() {
                let _ = tx.send(Decision::Stop);
            }
            stopped += 1;
        }
        stopped
    }

    /// Hold, or let go. Events keep queueing while a graph is held
    /// (SPEC §8.1), which is what makes it safe to rewire a live graph.
    pub fn hold(&self, run: Option<&str>, paused: bool) -> usize {
        let mut runs = self.runs.lock().unwrap();
        let mut changed = 0;
        for (id, live) in runs.iter_mut() {
            if run.is_some_and(|wanted| wanted != id) {
                continue;
            }
            live.paused.store(paused, Ordering::Relaxed);
            changed += 1;
        }
        changed
    }

    pub fn running(&self) -> Vec<String> {
        self.runs.lock().unwrap().keys().cloned().collect()
    }

    /// Tell the writer there is nothing more coming.
    pub fn finish(&self) {
        let _ = self.out.send(Outgoing::Done);
    }
}

/// Where perception models live.
///
/// `~/.local/share/cyberloom/models`, or `CYBERLOOM_MODELS`. Under the user's
/// own data folder rather than the workspace: weights are large, shared
/// between graphs, and have no business in a folder someone might commit.
fn models_folder() -> std::path::PathBuf {
    if let Some(set) = std::env::var_os("CYBERLOOM_MODELS") {
        return std::path::PathBuf::from(set);
    }
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    home.unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".local/share/cyberloom/models")
}

/// Which provider a graph's models should go through.
///
/// The graph names one; there is exactly one implemented, and a graph naming
/// another is told so by name rather than being run against a default that is
/// not what it asked for.
fn provider_for(graph: &graph_format::Graph) -> Result<Box<dyn ModelProvider>, RpcError> {
    // A block's own endpoint wins over the graph default, which is what lets
    // one graph reach two Ollamas.
    let endpoint = graph
        .blocks
        .iter()
        .filter(|b| b.kind == "llm")
        .find_map(|b| match b.settings.get("endpoint") {
            Some(graph_format::Setting::String(s)) if !s.trim().is_empty() => Some(s.clone()),
            _ => None,
        });

    match graph.defaults.provider.as_str() {
        "ollama" => Ok(Box::new(Ollama::new(endpoint.as_deref()))),
        other => Err(RpcError::new(
            "provider",
            format!(
                "`{other}` is not a provider this engine has yet; it speaks ollama. \
                 Change the graph's provider, or point its endpoint at an Ollama."
            ),
        )),
    }
}

/// Drain the queue onto the socket. One thread, so nothing interleaves.
pub fn write_lines(rx: Receiver<Outgoing>, mut write: impl std::io::Write) {
    for message in rx {
        match message {
            Outgoing::Done => break,
            Outgoing::Line(line) => {
                if writeln!(write, "{line}").is_err() || write.flush().is_err() {
                    // The shell hung up. Keep draining so nothing that is still
                    // sending blocks on a full channel; there is just nowhere
                    // for it to go.
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> (Arc<Session>, Receiver<Outgoing>) {
        let (tx, rx) = channel();
        (Arc::new(Session::new(tx)), rx)
    }

    fn lines(rx: &Receiver<Outgoing>) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        while let Ok(Outgoing::Line(line)) = rx.try_recv() {
            out.push(serde_json::from_str(&line).unwrap());
        }
        out
    }

    fn graph() -> graph_format::Graph {
        graph_format::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/graphs/customer-triage.loom"
        ))
        .unwrap()
    }

    /// A graph asking for a provider that does not exist is told which one it
    /// asked for, rather than quietly run against the one that does.
    #[test]
    fn an_unknown_provider_is_named() {
        let mut g = graph();
        g.defaults.provider = "openai".into();
        let Err(err) = provider_for(&g) else {
            panic!("openai is not a provider this engine has");
        };
        assert!(err.message.contains("openai"), "{}", err.message);
        assert!(err.message.contains("ollama"), "{}", err.message);
    }

    /// Replies and events share one queue, so a run's chatter can never split a
    /// reply in half.
    #[test]
    fn replies_and_events_go_out_whole_and_in_order() {
        let (session, rx) = session();
        session.send_reply(7, Reply::Error(RpcError::new("x", "first")));
        session.send_event(&RunEvent::WireActive {
            run: "r1".into(),
            wire: "w1".into(),
        });
        let out = lines(&rx);
        assert_eq!(out[0]["id"], 7);
        assert_eq!(out[1]["event"], "wire.active");
    }

    /// Stopping a run that is parked on a warning has to wake it, or the run
    /// thread waits forever for an answer nobody will send.
    #[test]
    fn stopping_wakes_a_run_that_is_waiting_to_be_asked() {
        let (session, rx) = session();
        session.runs.lock().unwrap().insert(
            "r1".into(),
            InFlight {
                cancel: Arc::new(AtomicBool::new(false)),
                paused: Arc::new(AtomicBool::new(false)),
                pending: HashMap::new(),
            },
        );

        let asker = Arc::clone(&session);
        let parked = std::thread::spawn(move || {
            asker.ask(
                "r1",
                &Warning {
                    block: "terminal".into(),
                    action: "Run `rm -rf /`".into(),
                    reason: "Running a shell command is a dangerous action.".into(),
                    remember: true,
                },
            )
        });

        // Wait for the warning to reach the queue, so the run really is parked.
        let warning = loop {
            if let Some(line) = lines(&rx).into_iter().find(|l| l["event"] == "run.warning") {
                break line;
            }
            std::thread::yield_now();
        };
        assert_eq!(warning["data"]["block"], "terminal");
        assert!(warning["data"]["remember"].as_bool().unwrap());

        assert_eq!(session.stop(Some("r1")), 1);
        assert_eq!(parked.join().unwrap(), Decision::Stop);
    }

    #[test]
    fn a_warning_is_answered_by_its_id() {
        let (session, rx) = session();
        session.runs.lock().unwrap().insert(
            "r1".into(),
            InFlight {
                cancel: Arc::new(AtomicBool::new(false)),
                paused: Arc::new(AtomicBool::new(false)),
                pending: HashMap::new(),
            },
        );
        let asker = Arc::clone(&session);
        let parked = std::thread::spawn(move || {
            asker.ask(
                "r1",
                &Warning {
                    block: "terminal".into(),
                    action: "Run `cargo build`".into(),
                    reason: "dangerous".into(),
                    remember: true,
                },
            )
        });
        let id = loop {
            if let Some(line) = lines(&rx).into_iter().find(|l| l["event"] == "run.warning") {
                break line["data"]["id"].as_str().unwrap().to_owned();
            }
            std::thread::yield_now();
        };
        assert!(session.answer(&id, Decision::ContinueAlways));
        assert_eq!(parked.join().unwrap(), Decision::ContinueAlways);
        // Answering it twice is a no-op, which is what a second click looks
        // like rather than an error.
        assert!(!session.answer(&id, Decision::Continue));
    }

    /// A shell that hangs up mid-warning leaves the run waiting on nobody. It
    /// stops rather than running the command unasked.
    #[test]
    fn a_vanished_shell_means_stop() {
        let (session, _rx) = session();
        // No run registered: the same shape as a run whose session is gone.
        assert_eq!(
            session.ask(
                "gone",
                &Warning {
                    block: "terminal".into(),
                    action: "Run `cargo build`".into(),
                    reason: "dangerous".into(),
                    remember: false,
                }
            ),
            Decision::Stop
        );
    }
}
