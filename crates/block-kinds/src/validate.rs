//! Checking a graph against the catalogue.
//!
//! Everything here is a *problem*, not a refusal. The application warns and
//! never blocks (SPEC §12.1): a graph with a problem still opens, still draws
//! and — where it can — still runs. What validation buys is that the user is
//! told, in the inspector and the status bar, rather than finding out when a
//! wire silently carries nothing.

use crate::{BlockKind, PortDef, kind};
use graph_format::{Graph, PortType, Side, Wire};

/// Something wrong with a graph, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// A block names a kind the catalogue does not have. Its wires are kept so
    /// that installing the missing block restores the graph.
    UnknownKind { block: String, kind: String },
    /// Two blocks, two frames, or two wires share an id.
    DuplicateId { id: String },
    /// A wire names something that is not on the canvas.
    MissingNode { wire: String, node: String },
    /// A wire names a port the node does not have.
    MissingPort {
        wire: String,
        node: String,
        port: String,
    },
    /// A wire runs the wrong way: out of an input, or into an output.
    WrongDirection {
        wire: String,
        node: String,
        port: String,
        side: Side,
    },
    /// A wire's two ends do not agree on type. This is the one the whole
    /// grammar exists to prevent (SPEC §4.1).
    Incompatible {
        wire: String,
        from: PortType,
        to: PortType,
    },
    /// A block sets something its kind does not define. Kept in the file, so a
    /// setting from a newer version survives a round trip through an older one.
    UnknownSetting { block: String, setting: String },
    /// A block names a frame that is not there.
    MissingFrame { block: String, frame: String },
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Problem::UnknownKind { block, kind } => {
                write!(
                    f,
                    "block `{block}` is a `{kind}`, which this build does not have"
                )
            }
            Problem::DuplicateId { id } => write!(f, "`{id}` is used more than once"),
            Problem::MissingNode { wire, node } => {
                write!(
                    f,
                    "wire `{wire}` names `{node}`, which is not on the canvas"
                )
            }
            Problem::MissingPort { wire, node, port } => {
                write!(f, "wire `{wire}`: `{node}` has no port `{port}`")
            }
            Problem::WrongDirection {
                wire,
                node,
                port,
                side,
            } => {
                let what = match side {
                    Side::In => "an input, so nothing can leave it",
                    Side::Out => "an output, so nothing can arrive at it",
                };
                write!(f, "wire `{wire}`: `{node}.{port}` is {what}")
            }
            Problem::Incompatible { wire, from, to } => {
                write!(f, "wire `{wire}`: {from} is not accepted by {to}")
            }
            Problem::UnknownSetting { block, setting } => {
                write!(
                    f,
                    "block `{block}` sets `{setting}`, which its kind does not define"
                )
            }
            Problem::MissingFrame { block, frame } => {
                write!(
                    f,
                    "block `{block}` is inside frame `{frame}`, which is not there"
                )
            }
        }
    }
}

/// The ports a loop frame exposes. A frame is not a block, but wires land on
/// it, so it needs a port list of its own (SPEC §8.4).
fn frame_ports() -> &'static [PortDef] {
    // The Loop kind in the catalogue is the same shape; a frame borrows it
    // rather than declaring a second copy that could drift.
    kind("loop").map(|k| k.ports).unwrap_or(&[])
}

fn node_ports<'a>(graph: &Graph, id: &str) -> Option<&'a [PortDef]> {
    if let Some(block) = graph.block(id) {
        // A custom block's ports come from its own code, not the catalogue,
        // so they cannot be checked here; that is the parser's job.
        if block.kind == "custom" {
            return None;
        }
        return kind(&block.kind).map(|k| k.ports);
    }
    if graph.frame(id).is_some() {
        return Some(frame_ports());
    }
    None
}

fn find_port<'a>(ports: &'a [PortDef], name: &str, side: Side) -> Option<&'a PortDef> {
    ports.iter().find(|p| p.name == name && p.side == side)
}

/// Check a graph. The list is ordered the way it is reported: identity first,
/// then wires, then settings, because a wrong id makes every later message
/// confusing.
pub fn validate(graph: &Graph) -> Vec<Problem> {
    let mut problems = Vec::new();

    // ---- identity
    let mut seen: Vec<&str> = Vec::new();
    for id in graph
        .blocks
        .iter()
        .map(|b| b.id.as_str())
        .chain(graph.frames.iter().map(|f| f.id.as_str()))
        .chain(graph.wires.iter().map(|w| w.id.as_str()))
    {
        if seen.contains(&id) {
            problems.push(Problem::DuplicateId { id: id.to_owned() });
        } else {
            seen.push(id);
        }
    }

    for block in &graph.blocks {
        if block.kind != "custom" && kind(&block.kind).is_none() {
            problems.push(Problem::UnknownKind {
                block: block.id.clone(),
                kind: block.kind.clone(),
            });
        }
        if let Some(frame) = &block.frame
            && graph.frame(frame).is_none()
        {
            problems.push(Problem::MissingFrame {
                block: block.id.clone(),
                frame: frame.clone(),
            });
        }
    }

    // ---- wires
    for wire in &graph.wires {
        let from = resolve(
            graph,
            wire,
            &wire.from.node,
            &wire.from.port,
            Side::Out,
            &mut problems,
        );
        let to = resolve(
            graph,
            wire,
            &wire.to.node,
            &wire.to.port,
            Side::In,
            &mut problems,
        );
        if let (Some(from), Some(to)) = (from, to)
            && !from.accepted_by(to)
        {
            problems.push(Problem::Incompatible {
                wire: wire.id.clone(),
                from,
                to,
            });
        }
    }

    // ---- settings
    for block in &graph.blocks {
        let Some(k) = kind(&block.kind) else { continue };
        if block.kind == "custom" {
            continue;
        }
        for name in block.settings.keys() {
            if k.setting(name).is_none() {
                problems.push(Problem::UnknownSetting {
                    block: block.id.clone(),
                    setting: name.clone(),
                });
            }
        }
    }

    problems
}

/// Find the type of one end of a wire, reporting why it could not be found.
fn resolve(
    graph: &Graph,
    wire: &Wire,
    node: &str,
    port: &str,
    side: Side,
    problems: &mut Vec<Problem>,
) -> Option<PortType> {
    // A custom block declares its ports on itself.
    if let Some(block) = graph.block(node)
        && block.kind == "custom"
    {
        return match block
            .ports
            .iter()
            .find(|p| p.name == port && p.side == side)
        {
            Some(p) => Some(p.port_type),
            None => {
                if block.ports.iter().any(|p| p.name == port) {
                    problems.push(Problem::WrongDirection {
                        wire: wire.id.clone(),
                        node: node.to_owned(),
                        port: port.to_owned(),
                        side: flip(side),
                    });
                } else {
                    problems.push(Problem::MissingPort {
                        wire: wire.id.clone(),
                        node: node.to_owned(),
                        port: port.to_owned(),
                    });
                }
                None
            }
        };
    }

    let Some(ports) = node_ports(graph, node) else {
        if graph.block(node).is_none() && graph.frame(node).is_none() {
            problems.push(Problem::MissingNode {
                wire: wire.id.clone(),
                node: node.to_owned(),
            });
        }
        // A block of an unknown kind was already reported; do not pile on.
        return None;
    };

    match find_port(ports, port, side) {
        Some(p) => Some(p.port_type),
        None => {
            if find_port(ports, port, flip(side)).is_some() {
                problems.push(Problem::WrongDirection {
                    wire: wire.id.clone(),
                    node: node.to_owned(),
                    port: port.to_owned(),
                    side: flip(side),
                });
            } else {
                problems.push(Problem::MissingPort {
                    wire: wire.id.clone(),
                    node: node.to_owned(),
                    port: port.to_owned(),
                });
            }
            None
        }
    }
}

fn flip(side: Side) -> Side {
    match side {
        Side::In => Side::Out,
        Side::Out => Side::In,
    }
}

/// Convenience for a `BlockKind`'s ports on one side, used by the library panel.
pub fn ports_on(k: &BlockKind, side: Side) -> impl Iterator<Item = &PortDef> {
    k.ports.iter().filter(move |p| p.side == side)
}
