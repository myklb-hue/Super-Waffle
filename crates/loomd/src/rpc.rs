//! The protocol between the shell and the engine.
//!
//! One JSON object per line, in both directions. Line-delimited rather than
//! length-prefixed because it is readable: `socat` on the socket shows you
//! exactly what the shell asked for, which matters a great deal when the engine
//! is a separate process from day one (PLAN §7, assumption 4).
//!
//! Requests carry an `id` and get exactly one response with the same `id`.
//! Events carry no id and arrive whenever the engine has something to say;
//! a client that does not understand an event must ignore it rather than fail,
//! so the engine can gain events without breaking an older shell.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Anything the shell sends.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "method", content = "params", rename_all = "camelCase")]
pub enum Request {
    /// What the engine is and what it can reach.
    #[serde(rename = "engine.status")]
    EngineStatus,
    /// Every built-in block kind. The shell has a generated copy for its own
    /// types, but the running engine is what decides what can actually be run.
    #[serde(rename = "graph.catalogue")]
    GraphCatalogue,
    /// The graphs in the workspace, by relative path.
    #[serde(rename = "workspace.list")]
    WorkspaceList,
    /// What the workspace chose, and what is actually installed.
    #[serde(rename = "workspace.settings")]
    WorkspaceSettings,
    /// Change what the workspace chose. The probe is not writable: it is a
    /// description of the machine, not a preference.
    #[serde(rename = "workspace.configure")]
    WorkspaceConfigure {
        settings: Box<crate::settings::WorkspaceSettings>,
    },
    /// Fetch a model's weights, explicitly and resumably (SPEC §15.13).
    ///
    /// Explicit because it is a request: nothing downloads because a graph
    /// happened to run. Progress arrives as console events, and asking again
    /// for something interrupted continues rather than starts over.
    #[serde(rename = "models.fetch")]
    ModelsFetch {
        url: String,
        /// Where to put it, relative to the models folder.
        name: String,
    },
    /// Read one graph.
    #[serde(rename = "graph.open")]
    GraphOpen { path: String },
    /// Write one graph back, in canonical form.
    ///
    /// The shell sends the whole graph rather than a patch. It is a few
    /// kilobytes, it makes the write atomic from the shell's point of view,
    /// and it means the engine never has to reconstruct a document from a
    /// stream of edits it might have missed.
    #[serde(rename = "graph.save")]
    GraphSave {
        path: String,
        graph: Box<graph_format::Graph>,
    },

    /// Run a graph. Answers immediately with the run's id; everything the run
    /// has to say arrives afterwards as events carrying that id.
    ///
    /// The graph comes from the shell rather than from disk, so what runs is
    /// what is on screen. Autosave means the two are almost always the same,
    /// but "almost always" is not a thing to build a Run button on: a person
    /// who edits and immediately runs should get the edit.
    #[serde(rename = "run.start")]
    RunStart {
        path: String,
        graph: Box<graph_format::Graph>,
    },

    /// Stop a run, or every run on this connection when given none.
    #[serde(rename = "run.stop")]
    RunStop {
        #[serde(default)]
        run: Option<String>,
    },

    /// Answer a warning the run is parked on (SPEC §12.1).
    #[serde(rename = "run.continue")]
    RunContinue {
        warning: String,
        decision: crate::run::runner::Decision,
    },

    /// Hold a live graph, or let it go (SPEC §8.1).
    ///
    /// Events keep queueing while it is held; nothing runs. That is what makes
    /// it safe to rewire a graph that is armed.
    #[serde(rename = "run.pause")]
    RunPause {
        #[serde(default)]
        run: Option<String>,
        paused: bool,
    },

    /// Re-read a custom block's signature (SPEC §10.3).
    ///
    /// The code comes from the shell rather than from disk for the same reason
    /// `run.start` does: a block is re-parsed on every save and every editor
    /// blur, and what the person is looking at is what should be parsed.
    #[serde(rename = "block.reparse")]
    BlockReparse {
        language: graph_format::Language,
        code: String,
        /// Which function, when the file declares several (SPEC §10.5). None
        /// takes the first.
        #[serde(default)]
        function: Option<String>,
    },
}

/// The envelope a request arrives in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub id: u64,
    #[serde(flatten)]
    pub request: Request,
}

/// Anything the engine sends back.
///
/// Serialise only: the engine writes replies and the shell, which is
/// TypeScript, reads them. Nothing in Rust parses one back, and the catalogue
/// inside it is a table of `&'static str` that could not be parsed anyway.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "result", content = "data", rename_all = "camelCase")]
pub enum Reply {
    EngineStatus(EngineStatus),
    Catalogue(Vec<block_kinds::BlockKind>),
    Workspace(Vec<GraphSummary>),
    WorkspaceInfo(Box<WorkspaceInfo>),
    Graph(Box<OpenGraph>),
    Saved(Saved),
    Running(RunStarted),
    /// What the code says the block's interface is, or why it could not be
    /// read. A failure is a reply, not an error: the block shows the line
    /// number and keeps its previous interface (SPEC §10.4).
    Interface(Reparsed),
    /// How many runs stopped, or whether a warning was answered.
    Acknowledged(Acknowledged),
    /// Not an exception: the shell shows it and carries on (SPEC §12.1).
    Error(RpcError),
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RunStarted {
    pub run: String,
    /// What the plan could not do. The run starts anyway (SPEC §12.1).
    pub problems: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Reparsed {
    /// Every function the file declares, in the order it declares them.
    pub blocks: Vec<block_source::Interface>,
    /// The one the request asked for, or the first.
    pub chosen: Option<block_source::Interface>,
    pub error: Option<block_source::SourceError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Acknowledged {
    pub ok: bool,
    pub count: u32,
}

/// The response envelope.
#[derive(Debug, Clone, Serialize)]
pub struct ReplyEnvelope {
    pub id: u64,
    #[serde(flatten)]
    pub reply: Reply,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RpcError {
    /// A stable key the shell can branch on.
    pub code: String,
    /// One sentence, already fit to show a person.
    pub message: String,
}

impl RpcError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub version: String,
    /// The workspace folder this engine is serving.
    pub workspace: String,
    /// How many graphs it can see.
    pub graphs: u32,
    /// The catalogue's size, so a shell can tell at a glance whether it is
    /// talking to an engine that knows the same blocks it does.
    pub kinds: u32,
}

/// What the settings screen draws: what was chosen, and what is there.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    /// The folder itself, so the screen can say which workspace this is.
    pub root: String,
    pub settings: crate::settings::WorkspaceSettings,
    pub probe: crate::settings::Probe,
}

/// One row of the workspace list. Enough to draw a tab without opening the file.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphSummary {
    /// Relative to the workspace root, so it is the same on any machine.
    pub path: String,
    pub id: String,
    pub name: String,
    pub blocks: u32,
    pub wires: u32,
    pub run_mode: graph_format::RunMode,
}

/// What a save produced.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Saved {
    pub path: String,
    /// The canonical form the engine wrote. Positions come back snapped to the
    /// grid, so the shell can adopt what is actually on disk rather than
    /// holding a document that differs from the file by a rounding.
    pub graph: Box<graph_format::Graph>,
    pub problems: Vec<String>,
    /// False when the file already held these exact bytes. The shell uses it to
    /// avoid saying "saved" when nothing changed.
    pub written: bool,
}

/// A graph, with everything the shell needs to draw it and nothing it would
/// have to ask for straight afterwards.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OpenGraph {
    pub path: String,
    pub graph: graph_format::Graph,
    /// What is wrong with it, if anything. A graph with problems still opens
    /// and still draws; the shell shows these rather than refusing the file.
    pub problems: Vec<String>,
}
