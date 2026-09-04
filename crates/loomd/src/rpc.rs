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
    Graph(Box<OpenGraph>),
    Saved(Saved),
    /// Not an exception: the shell shows it and carries on (SPEC §12.1).
    Error(RpcError),
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
