//! What happens to a block when its code changes (SPEC §10.3).
//!
//! > Ports that still exist keep their wires. A removed port drops its wire
//! > and says so in the console. A new port appears with a `+1 port` note.
//!
//! The rule is about *identity*: a port is the same port if it has the same
//! name and side. Not the same type — retyping a port keeps its wires, and
//! whether the wire is still legal is the grammar's business, reported like
//! any other problem rather than silently repaired. And not the same position,
//! because reordering parameters is a thing people do to code that is working.

use crate::Interface;
use graph_format::{Block, Graph, Port, Side, Wire};

/// What a reload did.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Reload {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    /// Ports whose type changed, with the wires that now cross a type they
    /// did not before. Kept, and reported.
    pub retyped: Vec<String>,
    /// Wires dropped because the port they landed on is gone.
    pub dropped: Vec<Wire>,
}

impl Reload {
    pub fn is_quiet(&self) -> bool {
        self.added.is_empty()
            && self.removed.is_empty()
            && self.retyped.is_empty()
            && self.dropped.is_empty()
    }

    /// The note the interface section and the status bar show: `+1 port`.
    pub fn note(&self) -> Option<String> {
        let mut parts = Vec::new();
        if !self.added.is_empty() {
            parts.push(format!(
                "+{} port{}",
                self.added.len(),
                plural(self.added.len())
            ));
        }
        if !self.removed.is_empty() {
            parts.push(format!(
                "−{} port{}",
                self.removed.len(),
                plural(self.removed.len())
            ));
        }
        (!parts.is_empty()).then(|| parts.join(" · "))
    }

    /// One sentence per dropped wire, for the console.
    pub fn messages(&self) -> Vec<String> {
        let mut out = Vec::new();
        for port in &self.removed {
            out.push(format!("`{port}` is gone from the signature"));
        }
        for wire in &self.dropped {
            out.push(format!(
                "dropped {} → {}: the port it landed on no longer exists",
                wire.from.to_ref(),
                wire.to.to_ref()
            ));
        }
        for port in &self.retyped {
            out.push(format!(
                "`{port}` changed type; its wires are kept and checked like any other"
            ));
        }
        out
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// Apply a freshly parsed interface to the block, and to the wires around it.
///
/// The graph is edited in place because a reload is one change: a shell that
/// applied the ports and then removed the wires would render a frame in which
/// a wire hangs off a port that is not there.
pub fn apply(graph: &mut Graph, block_id: &str, interface: &Interface) -> Reload {
    let Some(index) = graph.blocks.iter().position(|b| b.id == block_id) else {
        return Reload::default();
    };

    let before: Vec<Port> = graph.blocks[index].ports.clone();
    let after = interface.ports.clone();

    let key = |p: &Port| (p.name.clone(), p.side);
    let had: Vec<(String, Side)> = before.iter().map(key).collect();
    let has: Vec<(String, Side)> = after.iter().map(key).collect();

    let mut reload = Reload {
        added: after
            .iter()
            .filter(|p| !had.contains(&key(p)))
            .map(|p| p.name.clone())
            .collect(),
        removed: before
            .iter()
            .filter(|p| !has.contains(&key(p)))
            .map(|p| p.name.clone())
            .collect(),
        retyped: after
            .iter()
            .filter(|p| {
                before
                    .iter()
                    .any(|was| key(was) == key(p) && was.port_type != p.port_type)
            })
            .map(|p| p.name.clone())
            .collect(),
        dropped: Vec::new(),
    };

    // A wire survives if the port it lands on is still there, whatever its
    // type now is.
    let surviving: Vec<&str> = after.iter().map(|p| p.name.as_str()).collect();
    let (kept, dropped): (Vec<Wire>, Vec<Wire>) = graph.wires.iter().cloned().partition(|w| {
        let touches = |end: &graph_format::Endpoint| {
            end.node == block_id && !surviving.contains(&end.port.as_str())
        };
        !touches(&w.from) && !touches(&w.to)
    });
    reload.dropped = dropped;
    graph.wires = kept;

    let block = &mut graph.blocks[index];
    block.ports = after;
    // A setting whose parameter is gone goes with it: the file should say
    // nothing where the code asks for nothing.
    let declared: Vec<&str> = interface.settings.iter().map(|s| s.name.as_str()).collect();
    block
        .settings
        .retain(|name, _| declared.contains(&name.as_str()));
    // The function's own name is the block's title, unless the user renamed it
    // — a title someone typed is theirs, and a reload does not take it back.
    if block.title.is_none() || was_the_old_name(block, &before, interface) {
        block.title = Some(interface.name.clone());
    }

    reload
}

/// Whether the block's title is just the name the code used to have, in which
/// case renaming the function renames the block.
fn was_the_old_name(block: &Block, _before: &[Port], interface: &Interface) -> bool {
    // Only the title matching *some* function name is checkable here; a title
    // equal to the new name is already right, and one the user typed is not
    // going to be the identifier a parser produced.
    block
        .title
        .as_deref()
        .is_some_and(|t| t == interface.name || is_identifier(t))
}

fn is_identifier(text: &str) -> bool {
    !text.is_empty()
        && !text.contains(' ')
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use graph_format::{Endpoint, Language};

    fn door_watch() -> Graph {
        graph_format::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/graphs/door-watch.loom"
        ))
        .unwrap()
    }

    fn reparse(graph: &mut Graph, id: &str, code: &str) -> Reload {
        let interface = parse(Language::Python, code).unwrap().remove(0);
        apply(graph, id, &interface)
    }

    /// A port is the same port by name and side, not by position: reordering
    /// parameters is a thing people do to code that is working.
    #[test]
    fn the_same_ports_in_a_different_order_change_nothing() {
        let mut graph = door_watch();
        let before = graph.wires.clone();
        let reload = reparse(
            &mut graph,
            "door-check",
            "def door_check(frame: Image, threshold: float = 0.6) -> Data:\n    pass\n",
        );
        // The fixture declares a `memory` port its code does not, so this
        // reload removes it — and that is the only change.
        assert_eq!(reload.removed, ["memory"]);
        assert!(reload.added.is_empty());
        assert_eq!(graph.wires, before, "no wire touched `memory`");
    }

    /// Giving a parameter a default turns a port into a setting, which is a
    /// removal as far as the wires are concerned.
    #[test]
    fn a_parameter_that_gains_a_default_stops_being_a_port() {
        let mut graph = door_watch();
        let reload = reparse(
            &mut graph,
            "door-check",
            "def door_check(frame: Image = None, threshold: float = 0.6) -> Data:\n    pass\n",
        );
        assert!(reload.removed.contains(&"frame".to_owned()));
        assert_eq!(reload.dropped.len(), 1);
        let block = graph.blocks.iter().find(|b| b.id == "door-check").unwrap();
        assert!(!block.ports.iter().any(|p| p.name == "frame"));
    }

    /// A removed port drops its wire and says so (SPEC §10.3).
    #[test]
    fn a_removed_port_drops_its_wire_and_says_which() {
        let mut graph = door_watch();
        let reload = reparse(
            &mut graph,
            "door-check",
            "def door_check(threshold: float = 0.6) -> Data:\n    pass\n",
        );
        assert!(reload.removed.contains(&"frame".to_owned()));
        assert_eq!(reload.dropped.len(), 1);
        assert_eq!(reload.dropped[0].from, Endpoint::new("webcam", "frames"));
        assert!(!graph.wires.iter().any(|w| w.to.port == "frame"));
        // And the console gets a sentence naming both ends.
        let said = reload.messages().join("\n");
        assert!(said.contains("webcam.frames"), "{said}");
        assert!(said.contains("no longer exists"), "{said}");
    }

    /// A new port appears with the note the interface section shows.
    #[test]
    fn a_new_port_carries_its_note() {
        let mut graph = door_watch();
        let reload = reparse(
            &mut graph,
            "door-check",
            "def door_check(frame: Image, memory: Memory, threshold: float = 0.6) -> Data:\n    pass\n",
        );
        assert!(reload.added.is_empty(), "memory was already declared");
        // Removing it and adding it back is what produces a note.
        let mut graph = door_watch();
        reparse(
            &mut graph,
            "door-check",
            "def door_check(frame: Image) -> Data:\n    pass\n",
        );
        let reload = reparse(
            &mut graph,
            "door-check",
            "def door_check(frame: Image, memory: Memory) -> Data:\n    pass\n",
        );
        assert_eq!(reload.added, ["memory"]);
        assert_eq!(reload.note().as_deref(), Some("+1 port"));
    }

    /// Retyping a port keeps its wires. Whether the wire is still legal is the
    /// grammar's business, reported rather than silently repaired.
    #[test]
    fn retyping_a_port_keeps_its_wires_and_reports_it() {
        let mut graph = door_watch();
        let before = graph.wires.len();
        let reload = reparse(
            &mut graph,
            "door-check",
            "def door_check(frame: Text, threshold: float = 0.6) -> Data:\n    pass\n",
        );
        assert_eq!(reload.retyped, ["frame"]);
        assert_eq!(graph.wires.len(), before);
        assert!(reload.messages().iter().any(|m| m.contains("changed type")));
    }

    /// A setting whose parameter is gone goes with it: the file says nothing
    /// where the code asks for nothing.
    #[test]
    fn a_setting_with_no_parameter_left_is_dropped() {
        let mut graph = door_watch();
        assert!(
            graph
                .blocks
                .iter()
                .any(|b| b.settings.contains_key("threshold"))
        );
        reparse(
            &mut graph,
            "door-check",
            "def door_check(frame: Image) -> Data:\n    pass\n",
        );
        let block = graph.blocks.iter().find(|b| b.id == "door-check").unwrap();
        assert!(!block.settings.contains_key("threshold"));
    }

    /// Renaming the function renames the block — but not over a title someone
    /// typed themselves.
    #[test]
    fn renaming_the_function_renames_the_block_but_not_over_a_persons_title() {
        let mut graph = door_watch();
        reparse(
            &mut graph,
            "door-check",
            "def front_door(frame: Image) -> Data:\n    pass\n",
        );
        let block = graph.blocks.iter().find(|b| b.id == "door-check").unwrap();
        assert_eq!(block.title.as_deref(), Some("front_door"));

        graph
            .blocks
            .iter_mut()
            .find(|b| b.id == "door-check")
            .unwrap()
            .title = Some("Front door watcher".into());
        reparse(
            &mut graph,
            "door-check",
            "def something_else(frame: Image) -> Data:\n    pass\n",
        );
        let block = graph.blocks.iter().find(|b| b.id == "door-check").unwrap();
        assert_eq!(block.title.as_deref(), Some("Front door watcher"));
    }

    /// A reload that changes nothing says nothing.
    #[test]
    fn an_unchanged_signature_is_quiet() {
        let mut graph = door_watch();
        let code = "def door_check(frame: Image, memory: Memory, threshold: float = 0.6) -> Data:\n    pass\n";
        reparse(&mut graph, "door-check", code);
        let again = reparse(&mut graph, "door-check", code);
        assert!(again.is_quiet(), "{again:?}");
        assert!(again.note().is_none());
    }
}
