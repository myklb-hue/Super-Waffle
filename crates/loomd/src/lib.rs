//! The Cyberloom engine daemon.
//!
//! A separate process from the shell from the first build, so that Deploy as a
//! headless service (SPEC §15.1) is this without a window, and so the shell
//! surviving an engine crash is the normal case rather than a later retrofit.
//!
pub mod rpc;
pub mod run;
pub mod session;
pub mod workspace;

pub use rpc::{
    EngineStatus, Envelope, GraphSummary, OpenGraph, Reply, ReplyEnvelope, Request, RpcError, Saved,
};
pub use workspace::{Workspace, WorkspaceError};

use crate::session::{Outgoing, Session, write_lines};
use std::io::{BufRead, BufReader, Write};
use std::sync::Arc;
use std::sync::mpsc::channel;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Everything the engine knows. One workspace per engine; a second workspace is
/// a second engine, which is what makes both cheap to reason about.
pub struct Engine {
    workspace: Workspace,
}

impl Engine {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// Answer one request. Infallible on purpose: a failure is a `Reply::Error`
    /// the shell can show, not something that drops the connection.
    ///
    /// The session is where a run sends what it has to say, so it is a
    /// parameter rather than something the engine owns: one engine serves many
    /// connections, and a run belongs to the connection that asked for it.
    pub fn handle(&self, request: Request, session: &Arc<Session>) -> Reply {
        match request {
            Request::EngineStatus => match self.workspace.graphs() {
                Ok(graphs) => Reply::EngineStatus(EngineStatus {
                    version: VERSION.to_owned(),
                    workspace: self.workspace.root().display().to_string(),
                    graphs: graphs.len() as u32,
                    kinds: block_kinds::KINDS.len() as u32,
                }),
                Err(e) => Reply::Error(RpcError::new("workspace", e.to_string())),
            },

            Request::GraphCatalogue => Reply::Catalogue(block_kinds::KINDS.to_vec()),

            Request::WorkspaceList => match self.workspace.graphs() {
                Err(e) => Reply::Error(RpcError::new("workspace", e.to_string())),
                Ok(paths) => {
                    let mut out = Vec::new();
                    for path in paths {
                        let relative = self.workspace.relative(&path);
                        // A graph that will not parse is still listed, with what
                        // is known of it, so the workspace does not silently
                        // lose a file the user can see in their folder.
                        match graph_format::load(&path) {
                            Ok(g) => out.push(GraphSummary {
                                path: relative,
                                id: g.id,
                                name: g.name,
                                blocks: g.blocks.len() as u32,
                                wires: g.wires.len() as u32,
                                run_mode: g.run_mode,
                            }),
                            Err(_) => out.push(GraphSummary {
                                path: relative.clone(),
                                id: relative.clone(),
                                name: relative,
                                blocks: 0,
                                wires: 0,
                                run_mode: graph_format::RunMode::Once,
                            }),
                        }
                    }
                    Reply::Workspace(out)
                }
            },

            Request::GraphSave { path, graph } => match self.workspace.save(&path, &graph) {
                Err(e) => Reply::Error(RpcError::new("save", e.to_string())),
                Ok(written) => {
                    // Read back what was written rather than echoing what was
                    // sent: the canonical form is the truth, and the shell
                    // should adopt it instead of drifting from the file.
                    match self.workspace.load(&path) {
                        Err(e) => Reply::Error(RpcError::new("save", e.to_string())),
                        Ok(graph) => {
                            let problems = block_kinds::validate(&graph)
                                .iter()
                                .map(|p| p.to_string())
                                .collect();
                            Reply::Saved(Saved {
                                path,
                                graph: Box::new(graph),
                                problems,
                                written,
                            })
                        }
                    }
                }
            },

            Request::RunStart { path, graph } => {
                // Validation is reported, never a refusal: a graph with
                // problems still runs, and the console says what they were
                // (SPEC §12.1).
                let mut problems: Vec<String> = block_kinds::validate(&graph)
                    .iter()
                    .map(|p| p.to_string())
                    .collect();
                problems.extend(crate::run::plan(&graph).problems);
                // Relative paths in a graph's settings resolve against the
                // workspace, not the graph's own folder: the format says a
                // source path is "relative to the workspace folder, so a graph
                // moves with its files" (`graph_format::model::Source`).
                let _ = &path;
                match session.start_run(*graph, self.workspace.root().to_path_buf()) {
                    Ok(run) => Reply::Running(rpc::RunStarted { run, problems }),
                    Err(e) => Reply::Error(e),
                }
            }

            Request::BlockReparse {
                language,
                code,
                function,
            } => {
                // A parse failure is a reply rather than an error, because the
                // block goes red and keeps working with what it had — a graph
                // does not stop because someone is mid-keystroke (SPEC §10.4).
                let reparsed = match block_source::parse(language, &code) {
                    Err(error) => rpc::Reparsed {
                        blocks: Vec::new(),
                        chosen: None,
                        error: Some(error),
                    },
                    Ok(blocks) => {
                        let chosen = function
                            .as_deref()
                            .and_then(|name| blocks.iter().find(|b| b.name == name))
                            .or_else(|| blocks.first())
                            .cloned();
                        rpc::Reparsed {
                            blocks,
                            chosen,
                            error: None,
                        }
                    }
                };
                Reply::Interface(reparsed)
            }

            Request::RunStop { run } => Reply::Acknowledged(rpc::Acknowledged {
                ok: true,
                count: session.stop(run.as_deref()) as u32,
            }),

            Request::RunPause { run, paused } => Reply::Acknowledged(rpc::Acknowledged {
                ok: true,
                count: session.hold(run.as_deref(), paused) as u32,
            }),

            Request::RunContinue { warning, decision } => {
                let ok = session.answer(&warning, decision);
                Reply::Acknowledged(rpc::Acknowledged {
                    ok,
                    count: u32::from(ok),
                })
            }

            Request::GraphOpen { path } => match self.workspace.load(&path) {
                Err(e) => Reply::Error(RpcError::new("open", e.to_string())),
                Ok(graph) => {
                    let problems = block_kinds::validate(&graph)
                        .iter()
                        .map(|p| p.to_string())
                        .collect();
                    Reply::Graph(Box::new(OpenGraph {
                        path,
                        graph,
                        problems,
                    }))
                }
            },
        }
    }

    /// Serve one connection until it closes.
    ///
    /// One request per line, one reply per line, plus events whenever a run has
    /// something to say. A line that will not parse gets an error reply rather
    /// than a dropped connection, because a shell that sent one bad message
    /// should be told, not disconnected.
    ///
    /// Writing happens on its own thread, draining a queue. That is what keeps
    /// a reply and a run's events from interleaving halfway through an object,
    /// and it is why this reads rather than reading *and* writing.
    pub fn serve(&self, read: impl std::io::Read, write: impl Write + Send + 'static) {
        let (tx, rx) = channel::<Outgoing>();
        let writer = std::thread::Builder::new()
            .name("loomd-writer".into())
            .spawn(move || write_lines(rx, write));
        let session = Arc::new(Session::new(tx));

        for line in BufReader::new(read).lines() {
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<Envelope>(&line) {
                Ok(envelope) => {
                    let reply = self.handle(envelope.request, &session);
                    session.send_reply(envelope.id, reply);
                }
                Err(e) => {
                    session.send_reply(0, Reply::Error(RpcError::new("protocol", e.to_string())))
                }
            }
        }

        // The shell hung up. Anything still running is stopped rather than left
        // to finish into a socket nobody is reading: a run exists to be watched.
        session.stop(None);
        session.finish();
        if let Ok(writer) = writer {
            let _ = writer.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        Engine::new(
            Workspace::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures")).unwrap(),
        )
    }

    /// Somewhere for the writer thread to put what it wrote.
    struct Collected(Arc<std::sync::Mutex<Vec<u8>>>);

    impl Write for Collected {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// A session with nowhere for events to go. These requests produce none;
    /// the ones that do are covered over a real connection in `tests/protocol`.
    pub(super) fn nowhere() -> Arc<Session> {
        Arc::new(Session::new(channel().0))
    }

    #[test]
    fn reports_what_it_is_serving() {
        let Reply::EngineStatus(status) = engine().handle(Request::EngineStatus, &nowhere()) else {
            panic!("expected a status");
        };
        assert_eq!(status.graphs, 4);
        assert_eq!(status.kinds as usize, block_kinds::KINDS.len());
        assert!(status.workspace.ends_with("fixtures"));
    }

    #[test]
    fn lists_the_workspace() {
        let Reply::Workspace(graphs) = engine().handle(Request::WorkspaceList, &nowhere()) else {
            panic!("expected a list");
        };
        assert_eq!(graphs.len(), 4);
        let triage = graphs.iter().find(|g| g.id == "customer-triage").unwrap();
        assert_eq!(triage.path, "graphs/customer-triage.loom");
        assert_eq!(triage.blocks, 6);
        assert_eq!(triage.wires, 5);
    }

    #[test]
    fn opens_a_graph_with_its_problems() {
        let Reply::Graph(open) = engine().handle(
            Request::GraphOpen {
                path: "graphs/home-assistant.loom".into(),
            },
            &nowhere(),
        ) else {
            panic!("expected a graph");
        };
        assert_eq!(open.graph.id, "home-assistant");
        assert!(open.problems.is_empty(), "{:?}", open.problems);
    }

    #[test]
    fn a_bad_path_is_an_error_reply_not_a_panic() {
        let Reply::Error(e) = engine().handle(
            Request::GraphOpen {
                path: "../etc/passwd".into(),
            },
            &nowhere(),
        ) else {
            panic!("expected an error");
        };
        assert_eq!(e.code, "open");
        assert!(e.message.contains("leaves the workspace"));
    }

    /// The protocol is line-delimited JSON so that it can be read by a person
    /// with `socat`; this is that, exercised end to end.
    #[test]
    fn speaks_one_json_object_per_line() {
        let input = concat!(
            r#"{"id":1,"method":"engine.status"}"#,
            "\n",
            r#"{"id":2,"method":"graph.open","params":{"path":"graphs/door-watch.loom"}}"#,
            "\n",
            "not json\n",
        );
        // The writer now runs on its own thread, so it needs somewhere to
        // write that outlives this frame.
        let out = Arc::new(std::sync::Mutex::new(Vec::new()));
        engine().serve(input.as_bytes(), Collected(Arc::clone(&out)));
        let written = std::mem::take(&mut *out.lock().unwrap());
        let lines: Vec<_> = String::from_utf8(written)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect();
        assert_eq!(lines.len(), 3);

        let first: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        assert_eq!(first["id"], 1);
        assert_eq!(first["result"], "engineStatus");

        let second: serde_json::Value = serde_json::from_str(&lines[1]).unwrap();
        assert_eq!(second["id"], 2);
        assert_eq!(second["data"]["graph"]["id"], "door-watch");

        // A line that will not parse is answered, not fatal.
        let third: serde_json::Value = serde_json::from_str(&lines[2]).unwrap();
        assert_eq!(third["result"], "error");
        assert_eq!(third["data"]["code"], "protocol");
    }

    #[test]
    fn serves_the_catalogue_the_engine_actually_has() {
        let Reply::Catalogue(kinds) = engine().handle(Request::GraphCatalogue, &nowhere()) else {
            panic!("expected a catalogue");
        };
        assert_eq!(kinds.len(), block_kinds::KINDS.len());
        assert!(kinds.iter().any(|k| k.id == "llm"));
    }
}

#[cfg(test)]
mod save_tests {
    use super::tests::nowhere;
    use super::*;

    /// The acceptance criterion for slice 3: a graph edited and saved comes
    /// back the same graph.
    #[test]
    fn a_saved_graph_reopens_identically() {
        let dir = std::env::temp_dir().join(format!("loomd-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/graphs/customer-triage.loom"
        );
        std::fs::copy(source, dir.join("g.loom")).unwrap();

        let engine = Engine::new(Workspace::open(&dir).unwrap());
        let Reply::Graph(open) = engine.handle(
            Request::GraphOpen {
                path: "g.loom".into(),
            },
            &nowhere(),
        ) else {
            panic!("expected a graph");
        };

        // Move a block by less than half a grid step and add one.
        let mut graph = open.graph.clone();
        graph.blocks[0].position.x += 4.0;
        let mut added = graph.blocks[0].clone();
        added.id = "second-input".into();
        added.position = graph_format::Position { x: 66.0, y: 704.0 };
        graph.blocks.push(added);

        let Reply::Saved(saved) = engine.handle(
            Request::GraphSave {
                path: "g.loom".into(),
                graph: Box::new(graph),
            },
            &nowhere(),
        ) else {
            panic!("expected a save");
        };
        assert!(saved.written);
        assert_eq!(saved.graph.blocks.len(), 7);
        // The nudge was inside half a grid step, so it snapped back.
        assert_eq!(
            saved.graph.blocks[0].position.x,
            open.graph.blocks[0].position.x
        );

        let Reply::Graph(reopened) = engine.handle(
            Request::GraphOpen {
                path: "g.loom".into(),
            },
            &nowhere(),
        ) else {
            panic!("expected a graph");
        };
        assert_eq!(reopened.graph, *saved.graph);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Autosave fires on every edit, so a save that would write the same bytes
    /// has to be free.
    #[test]
    fn saving_unchanged_bytes_writes_nothing() {
        let dir = std::env::temp_dir().join(format!("loomd-nochange-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/graphs/door-watch.loom"
        );
        std::fs::copy(source, dir.join("g.loom")).unwrap();

        let engine = Engine::new(Workspace::open(&dir).unwrap());
        let Reply::Graph(open) = engine.handle(
            Request::GraphOpen {
                path: "g.loom".into(),
            },
            &nowhere(),
        ) else {
            panic!("expected a graph");
        };
        let before = std::fs::metadata(dir.join("g.loom"))
            .unwrap()
            .modified()
            .unwrap();

        let Reply::Saved(saved) = engine.handle(
            Request::GraphSave {
                path: "g.loom".into(),
                graph: Box::new(open.graph.clone()),
            },
            &nowhere(),
        ) else {
            panic!("expected a save");
        };
        assert!(
            !saved.written,
            "an unchanged graph should not touch the file"
        );
        assert_eq!(
            std::fs::metadata(dir.join("g.loom"))
                .unwrap()
                .modified()
                .unwrap(),
            before
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_save_outside_the_workspace_is_refused() {
        let engine = Engine::new(
            Workspace::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures")).unwrap(),
        );
        let graph = match engine.handle(
            Request::GraphOpen {
                path: "graphs/door-watch.loom".into(),
            },
            &nowhere(),
        ) {
            Reply::Graph(open) => Box::new(open.graph),
            _ => panic!("expected a graph"),
        };
        let Reply::Error(e) = engine.handle(
            Request::GraphSave {
                path: "../escaped.loom".into(),
                graph,
            },
            &nowhere(),
        ) else {
            panic!("expected an error");
        };
        assert_eq!(e.code, "save");
    }
}
