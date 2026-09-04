//! The persisted shape of a graph: everything a `.loom` file holds.
//!
//! Run state, events and warnings are the engine's and are not here; a graph
//! file describes what to run, never a run.

use crate::types::{Language, OverlapPolicy, PortType, RunMode, Side, SourceMode, View};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::BTreeMap;

/// The grid the canvas snaps to, and the grid positions are rounded to on save
/// (SPEC §16.3). Held here because the writer enforces it.
pub const GRID: f64 = 22.0;

/// The only format version that exists. A file declaring anything else is
/// refused rather than guessed at.
pub const VERSION: u32 = 1;

/// One `.loom` file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Graph {
    pub version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub run_mode: RunMode,
    /// On by default. Turning it off allows remote model providers; the first
    /// send of a run to any remote service warns (SPEC §15.4).
    pub local_only: bool,
    pub execution: Execution,
    pub defaults: Defaults,
    pub overlap: Overlap,
    pub between: Between,
    /// Secret *names*, never values: values live in the OS keyring, so a graph
    /// is safe to commit (SPEC §12.4). Sorted, like every other map in the
    /// format: their order carries no meaning, and sorting is what lets two
    /// hands produce one file.
    pub env: BTreeMap<String, String>,
    pub blocks: Vec<Block>,
    pub frames: Vec<Frame>,
    pub wires: Vec<Wire>,
    pub ui: Ui,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Execution {
    pub runtime: String,
    pub concurrency: u32,
    pub timeout_sec: u32,
}

impl Default for Execution {
    fn default() -> Self {
        Self {
            runtime: "local".into(),
            concurrency: 4,
            timeout_sec: 120,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Defaults {
    pub provider: String,
    pub model: String,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            provider: "ollama".into(),
            model: "llama3.2:3b".into(),
        }
    }
}

/// What happens when an event arrives while the previous one is still running.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Overlap {
    pub policy: OverlapPolicy,
    pub max_queue: u32,
    pub coalesce_ms: u32,
    pub loop_parallel: u32,
}

impl Default for Overlap {
    fn default() -> Self {
        Self {
            policy: OverlapPolicy::Queue,
            max_queue: 100,
            coalesce_ms: 0,
            loop_parallel: 2,
        }
    }
}

/// What survives from one event to the next in a live graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Between {
    pub keep_state: bool,
    pub restart_on_crash: bool,
}

impl Default for Between {
    fn default() -> Self {
        Self {
            keep_state: true,
            restart_on_crash: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, Type)]
pub struct Ui {
    pub viewport: Viewport,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, Type)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Position {
    /// Positions are rounded to the grid on save, so a block nudged by a pixel
    /// does not show up as a diff.
    pub fn snapped(self) -> Self {
        Self {
            x: (self.x / GRID).round() * GRID,
            y: (self.y / GRID).round() * GRID,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
pub struct Size {
    pub w: f64,
    /// Height is only meaningful for the views that have one: stage and code.
    #[serde(default)]
    pub h: Option<f64>,
}

/// One block on the canvas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub id: String,
    /// A key into the built-in catalogue, or `custom`.
    pub kind: String,
    /// Overrides the kind's own title when the user renames the block.
    #[serde(default)]
    pub title: Option<String>,
    pub position: Position,
    #[serde(default)]
    pub size: Option<Size>,
    pub view: View,
    /// Validated against the kind's setting definitions.
    pub settings: BTreeMap<String, Value>,
    /// A custom block's parsed interface: the ports its signature produced.
    pub ports: Vec<Port>,
    #[serde(default)]
    pub source: Option<Source>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub breakpoint: bool,
    /// The loop frame that contains this block, if any.
    #[serde(default)]
    pub frame: Option<String>,
}

/// A port declared by a custom block, derived from its signature (SPEC §10.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Port {
    pub name: String,
    #[serde(rename = "type")]
    pub port_type: PortType,
    pub side: Side,
    #[serde(default)]
    pub optional: bool,
}

/// Where a custom block's code lives. Inline code is embedded in the file as a
/// block scalar so a diff reads as code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub mode: SourceMode,
    pub language: Language,
    #[serde(default)]
    pub code: Option<String>,
    /// Relative to the workspace folder, so a graph moves with its files.
    #[serde(default)]
    pub path: Option<String>,
}

/// One typed connection. Endpoints are written as `block.port`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Wire {
    pub id: String,
    pub from: Endpoint,
    pub to: Endpoint,
}

/// One end of a wire. It names a *node*, which is a block or a loop frame:
/// a frame has ports of its own (`items` in, `item`, `results`, `done` and
/// `errors` out), so wires land on frames as well as on blocks (SPEC §8.4).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
pub struct Endpoint {
    pub node: String,
    pub port: String,
}

impl Endpoint {
    pub fn new(node: impl Into<String>, port: impl Into<String>) -> Self {
        Self {
            node: node.into(),
            port: port.into(),
        }
    }

    /// `node.port`, the form the file uses.
    pub fn to_ref(&self) -> String {
        format!("{}.{}", self.node, self.port)
    }
}

/// A loop: a dashed frame on the canvas, not a card. Blocks inside it repeat
/// once per item (SPEC §8.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub id: String,
    pub kind: FrameKind,
    pub position: Position,
    pub size: Size,
    /// The port the frame iterates.
    pub over: Endpoint,
    /// The name each item takes inside the frame.
    #[serde(rename = "as")]
    pub as_name: String,
    pub parallel: u32,
    pub max: u32,
    #[serde(default)]
    pub stop_when: Option<Endpoint>,
    pub continue_on_error: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum FrameKind {
    #[default]
    Loop,
}

impl FrameKind {
    pub fn as_str(self) -> &'static str {
        match self {
            FrameKind::Loop => "loop",
        }
    }
}

/// A setting value. Deliberately small: a setting is a scalar, a list or a
/// record, and nothing in the format needs more than that.
///
/// `Int` is 32-bit rather than 64. A setting holds a port number, a count, a
/// timeout — never something that needs more — and JSON, which is what carries
/// these over the socket, has one number type anyway. A whole number too large
/// for `Int` is read as a `Float`, which is the same precision JSON would have
/// given it; because the writer prints an integral float without its point, the
/// file still round-trips byte for byte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i32),
    Float(f64),
    String(String),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_owned())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl Graph {
    pub fn block(&self, id: &str) -> Option<&Block> {
        self.blocks.iter().find(|b| b.id == id)
    }

    pub fn frame(&self, id: &str) -> Option<&Frame> {
        self.frames.iter().find(|f| f.id == id)
    }

    /// Whether an id names something a wire may connect to: a block or a
    /// loop frame.
    pub fn has_node(&self, id: &str) -> bool {
        self.block(id).is_some() || self.frame(id).is_some()
    }

    /// Every wire leaving a node.
    pub fn wires_from<'a>(&'a self, node: &'a str) -> impl Iterator<Item = &'a Wire> {
        self.wires.iter().filter(move |w| w.from.node == node)
    }

    /// Every wire arriving at a node.
    pub fn wires_into<'a>(&'a self, node: &'a str) -> impl Iterator<Item = &'a Wire> {
        self.wires.iter().filter(move |w| w.to.node == node)
    }
}
