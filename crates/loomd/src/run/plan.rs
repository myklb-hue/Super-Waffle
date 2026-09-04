//! Deciding what runs, in what order, and what merely stands ready.
//!
//! The plan is built once, before anything executes, from the graph alone. It
//! is a pure function of the file: the same graph always plans the same way,
//! which is what makes a run reproducible enough to argue about.
//!
//! # Steps and capabilities
//!
//! The distinction this module exists for. A Terminal wired into a Toolbox is
//! not a step in the program — nothing sends it a value and it produces none.
//! It is a *capability*: it is bound to whoever holds the handle, and it runs
//! only if that holder calls it. In the customer-triage example (SPEC §13.1)
//! three of the six blocks are capabilities, and a plan that scheduled them
//! would run `cargo build` before the model had decided it wanted to.
//!
//! A block is a capability when it has wired outputs and every one of them is
//! a *handle* — `tools` or `memory`, the types whose holder makes the calls
//! (SPEC §4.3). Not merely a closed type: `exec` is closed too, and it is
//! control flow rather than something to call, so a Branch whose only outputs
//! are `exec` is a step that sets other steps going. Reading `exec` as a handle
//! made every Branch a tool nobody held, and it never ran.
//!
//! A block with no wired outputs at all is a *step*, not a capability: it was
//! placed to do something, and having nowhere to send the result does not make
//! it a handle.
//!
//! Being a capability and being a step are independent. The assistant example
//! (§13.3) wires a model's thoughts into a Terminal and takes its `stdout`,
//! while the same Terminal is also in a Toolbox. That block runs in order *and*
//! answers calls.

use graph_format::{Block, Endpoint, Graph, PortType};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Everything the runner needs to execute a graph, worked out in advance.
#[derive(Debug, Clone)]
pub struct Plan {
    /// The steps, in the order they will run.
    pub order: Vec<String>,
    /// Blocks bound as capabilities: the tools a holder may call.
    pub capabilities: BTreeSet<String>,
    /// For each holder, the blocks whose handles it holds. A Toolbox's own
    /// slots are resolved through it, so an LLM's entry lists the runtimes
    /// rather than the Toolbox between them.
    pub bindings: BTreeMap<String, Vec<String>>,
    /// Which wire connects a producing port to a consuming one, so the runner
    /// can say which wire lit up.
    pub wires: HashMap<Endpoint, Vec<(String, Endpoint)>>,
    /// The blocks that emit on their own initiative, in file order. A graph
    /// with one of these never finishes by itself (SPEC §8.2).
    pub sources: Vec<String>,
    /// What the plan could not do. Reported, never fatal: a graph with a
    /// problem still runs, minus the part that could not be ordered
    /// (SPEC §12.1).
    pub problems: Vec<String>,
}

impl Plan {
    /// Whether this block answers calls rather than taking a turn.
    pub fn is_capability(&self, id: &str) -> bool {
        self.capabilities.contains(id)
    }

    /// The steps an event on this port sets going, in the order they run.
    ///
    /// A live graph does not run top to bottom; it runs the part of itself
    /// downstream of whatever just happened (SPEC §8.2). Two sources in one
    /// graph are two programs sharing a canvas, and this is what keeps a file
    /// arriving in a folder from also firing the fifteen-minute digest.
    ///
    /// Reachability follows every wire, `exec` included. Exec is control flow
    /// rather than a value (SPEC §4.3), and control flow is precisely what
    /// decides which blocks run — a Schedule's `tick` carries nothing and
    /// still sets the whole branch below it going.
    pub fn downstream_of(&self, node: &str, port: &str) -> Vec<String> {
        let mut reached: HashSet<String> = HashSet::new();
        let mut frontier: Vec<Endpoint> = vec![Endpoint::new(node, port)];
        while let Some(from) = frontier.pop() {
            let Some(targets) = self.wires.get(&from) else {
                continue;
            };
            for (_, to) in targets {
                if !reached.insert(to.node.clone()) {
                    continue;
                }
                // Everything that block produces carries on downstream.
                for end in self.wires.keys().filter(|e| e.node == to.node) {
                    frontier.push(end.clone());
                }
            }
        }
        self.order
            .iter()
            .filter(|id| reached.contains(*id))
            .cloned()
            .collect()
    }

    /// Every source in the graph, in file order.
    pub fn sources(&self) -> &[String] {
        &self.sources
    }
}

/// Build the plan for a Once-mode run.
pub fn plan(graph: &Graph) -> Plan {
    let by_id: HashMap<&str, &Block> = graph.blocks.iter().map(|b| (b.id.as_str(), b)).collect();
    let mut problems = Vec::new();

    // Where every wire goes, keyed by the port that produces the value.
    let mut wires: HashMap<Endpoint, Vec<(String, Endpoint)>> = HashMap::new();
    for wire in &graph.wires {
        wires
            .entry(wire.from.clone())
            .or_default()
            .push((wire.id.clone(), wire.to.clone()));
    }

    // ------------------------------------------------------ what is a handle
    let mut wired_out: HashMap<&str, Vec<PortType>> = HashMap::new();
    let mut wired_in: HashMap<&str, Vec<PortType>> = HashMap::new();
    for wire in &graph.wires {
        if let Some(t) = port_type(&by_id, &wire.from, Side::Out) {
            wired_out.entry(block_of(&wire.from)).or_default().push(t);
        }
        if let Some(t) = port_type(&by_id, &wire.to, Side::In) {
            wired_in.entry(block_of(&wire.to)).or_default().push(t);
        }
    }

    let capabilities: BTreeSet<String> = graph
        .blocks
        .iter()
        .filter(|b| {
            let outs = wired_out.get(b.id.as_str());
            match outs {
                // Nowhere to send a result is not the same as having no result.
                None => false,
                Some(types) => types.iter().all(|t| t.is_handle()),
            }
        })
        .map(|b| b.id.clone())
        .collect();

    // ------------------------------------------------------------- bindings
    //
    // Following a handle wire backwards from a holder gives the blocks it may
    // call. A Toolbox is transparent here: it is a bundle, so an LLM holding
    // its handle holds the runtimes behind it, and the model's tool list names
    // `terminal.run`, never `toolbox.something`.
    let mut bindings: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for block in &graph.blocks {
        let held = resolve_handles(graph, &by_id, &block.id, &mut HashSet::new());
        if !held.is_empty() && !capabilities.contains(&block.id) {
            bindings.insert(block.id.clone(), held);
        }
    }

    // ---------------------------------------------------------------- order
    //
    // Kahn's algorithm over the steps only, with ties broken by the block's
    // position in the file. Without that tiebreak the order would depend on
    // hash iteration and two runs of the same graph could log differently.
    let steps: Vec<&str> = graph
        .blocks
        .iter()
        .map(|b| b.id.as_str())
        .filter(|id| !capabilities.contains(*id))
        .collect();
    let step_set: HashSet<&str> = steps.iter().copied().collect();

    let mut waiting_on: HashMap<&str, HashSet<&str>> =
        steps.iter().map(|id| (*id, HashSet::new())).collect();
    for wire in &graph.wires {
        let (from, to) = (block_of(&wire.from), block_of(&wire.to));
        // A handle wire is a binding, not a dependency: the holder does not
        // wait for the tool, it is handed the ability to call it.
        let handle = port_type(&by_id, &wire.from, Side::Out).is_some_and(PortType::is_handle);
        if handle || !step_set.contains(from) || !step_set.contains(to) || from == to {
            continue;
        }
        waiting_on.entry(to).or_default().insert(from);
    }

    let mut order = Vec::new();
    let mut placed: HashSet<&str> = HashSet::new();
    loop {
        let ready: Vec<&str> = steps
            .iter()
            .copied()
            .filter(|id| !placed.contains(id))
            .filter(|id| waiting_on[id].iter().all(|dep| placed.contains(dep)))
            .collect();
        if ready.is_empty() {
            break;
        }
        for id in ready {
            placed.insert(id);
            order.push(id.to_owned());
        }
    }

    // Whatever is left is in a cycle. It does not stop the run: the acyclic
    // part still executes and the console says which blocks were left out.
    let stuck: Vec<&str> = steps
        .iter()
        .copied()
        .filter(|id| !placed.contains(id))
        .collect();
    if !stuck.is_empty() {
        problems.push(format!(
            "{} cannot be ordered: {} form a cycle, and Once mode has no way through one",
            if stuck.len() == 1 {
                "one block"
            } else {
                "blocks"
            },
            stuck.join(", ")
        ));
    }

    if !graph.frames.is_empty() {
        problems.push(format!(
            "{} loop frame{} will run once rather than repeating: frames arrive with live graphs",
            graph.frames.len(),
            if graph.frames.len() == 1 { "" } else { "s" }
        ));
    }

    let sources = graph
        .blocks
        .iter()
        .filter(|b| block_kinds::kind(&b.kind).is_some_and(|k| k.source))
        .map(|b| b.id.clone())
        .collect();

    let _ = wired_in;
    Plan {
        order,
        capabilities,
        bindings,
        wires,
        sources,
        problems,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    In,
    Out,
}

fn block_of(end: &Endpoint) -> &str {
    &end.node
}

/// The declared type of one end of a wire, or none when the endpoint names a
/// port the block does not have — which validation already reports, so the plan
/// steps over it rather than reporting it twice.
fn port_type(by_id: &HashMap<&str, &Block>, end: &Endpoint, side: Side) -> Option<PortType> {
    let block = by_id.get(end.node.as_str())?;
    // A custom block's ports come from its signature, not the catalogue.
    if let Some(p) = block.ports.iter().find(|p| {
        p.name == end.port
            && match side {
                Side::In => p.side == graph_format::Side::In,
                Side::Out => p.side == graph_format::Side::Out,
            }
    }) {
        return Some(p.port_type);
    }
    let kind = block_kinds::kind(&block.kind)?;
    kind.ports
        .iter()
        .find(|p| {
            p.name == end.port
                && match side {
                    Side::In => p.side == graph_format::Side::In,
                    Side::Out => p.side == graph_format::Side::Out,
                }
        })
        .map(|p| p.port_type)
}

/// The blocks whose handles `holder` ends up holding, following handle wires
/// backwards and seeing through bundles.
///
/// `seen` guards a Toolbox wired into itself. That is a graph the user can draw
/// and validation warns about; without the guard it would be a stack overflow
/// rather than a warning, and an engine that dies on a bad graph is worse than
/// one that says so.
fn resolve_handles(
    graph: &Graph,
    by_id: &HashMap<&str, &Block>,
    holder: &str,
    seen: &mut HashSet<String>,
) -> Vec<String> {
    if !seen.insert(holder.to_owned()) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for wire in &graph.wires {
        if wire.to.node != holder {
            continue;
        }
        if !port_type(by_id, &wire.from, Side::Out).is_some_and(PortType::is_handle) {
            continue;
        }
        let source = &wire.from.node;
        let is_bundle = by_id
            .get(source.as_str())
            .is_some_and(|b| matches!(b.kind.as_str(), "toolbox" | "memoryHub"));
        if is_bundle {
            out.extend(resolve_handles(graph, by_id, source, seen));
        } else if !out.contains(source) {
            out.push(source.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Graph {
        graph_format::load(format!(
            "{}/../../fixtures/graphs/{name}.loom",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    /// SPEC §13.1 in one assertion: of six blocks only three are steps, and the
    /// model holds the two runtimes through the Toolbox.
    #[test]
    fn the_triage_example_runs_three_of_its_six_blocks() {
        let plan = plan(&fixture("customer-triage"));
        assert_eq!(plan.order, ["input", "llm", "report"]);
        assert_eq!(
            plan.capabilities.iter().collect::<Vec<_>>(),
            ["python", "terminal", "toolbox"]
        );
        assert!(plan.problems.is_empty(), "{:?}", plan.problems);
    }

    /// The Toolbox is a bundle, so the model's tool list names the runtimes
    /// behind it rather than the box in front of them.
    #[test]
    fn a_toolbox_is_transparent_to_the_model_holding_it() {
        let plan = plan(&fixture("customer-triage"));
        assert_eq!(plan.bindings["llm"], ["terminal", "python"]);
        // The Toolbox holds them too, but it is a capability, not a holder,
        // so it gets no entry of its own to be confused with the model's.
        assert!(!plan.bindings.contains_key("toolbox"));
    }

    /// A block with nowhere to send its result is still a step. Deciding
    /// otherwise would silently drop a Terminal someone placed to run one
    /// command.
    #[test]
    fn a_block_with_no_wires_out_still_runs() {
        let mut graph = fixture("customer-triage");
        graph.wires.retain(|w| w.from.node != "terminal");
        let plan = plan(&graph);
        assert!(plan.order.contains(&"terminal".to_owned()));
        assert!(!plan.is_capability("terminal"));
    }

    /// A block that both answers calls and takes a turn is both, which is what
    /// the assistant example needs (SPEC §13.3).
    #[test]
    fn a_terminal_that_also_reports_is_a_step_as_well() {
        let mut graph = fixture("customer-triage");
        graph.wires.push(graph_format::Wire {
            id: "w6".into(),
            from: Endpoint::new("terminal", "stdout"),
            to: Endpoint::new("report", "value"),
        });
        let plan = plan(&graph);
        assert!(!plan.is_capability("terminal"));
        assert!(plan.order.contains(&"terminal".to_owned()));
        // It is still bound to the model, which can still call it.
        assert!(plan.bindings["llm"].contains(&"terminal".to_owned()));
        // And it is ordered before the block reading its output.
        let at = |id: &str| plan.order.iter().position(|b| b == id).unwrap();
        assert!(at("terminal") < at("report"));
    }

    /// A cycle is reported and the rest of the graph still runs, because
    /// warning is the policy and refusing is not (SPEC §12.1).
    ///
    /// The blast radius is the point: a cycle takes everything downstream of it
    /// with it, because those blocks are waiting on a value that will never
    /// arrive. Here a Convert feeds the model its own answer, so the model, the
    /// Convert and the report all fall out and only the Input is left.
    #[test]
    fn a_cycle_takes_what_is_downstream_of_it() {
        let mut graph = fixture("customer-triage");
        graph.blocks.push(graph_format::Block {
            id: "convert".into(),
            kind: "convert".into(),
            title: None,
            position: graph_format::Position { x: 0.0, y: 0.0 },
            size: None,
            view: graph_format::View::Compact,
            settings: Default::default(),
            ports: Vec::new(),
            source: None,
            disabled: false,
            breakpoint: false,
            frame: None,
        });
        graph.wires.push(graph_format::Wire {
            id: "w6".into(),
            from: Endpoint::new("llm", "text"),
            to: Endpoint::new("convert", "value"),
        });
        graph.wires.push(graph_format::Wire {
            id: "w7".into(),
            from: Endpoint::new("convert", "value"),
            to: Endpoint::new("llm", "prompt"),
        });

        let plan = plan(&graph);
        assert_eq!(plan.order, ["input"]);
        let cycle = plan
            .problems
            .iter()
            .find(|p| p.contains("cycle"))
            .expect("the cycle is reported");
        for block in ["llm", "convert", "report"] {
            assert!(cycle.contains(block), "{cycle}");
        }
    }

    /// A holder cannot make itself hold its own handle, and trying does not
    /// take the engine down with it.
    #[test]
    fn a_toolbox_wired_to_itself_terminates() {
        let mut graph = fixture("customer-triage");
        graph.wires.push(graph_format::Wire {
            id: "w6".into(),
            from: Endpoint::new("toolbox", "tools"),
            to: Endpoint::new("toolbox", "tools"),
        });
        let plan = plan(&graph);
        assert_eq!(plan.bindings["llm"], ["terminal", "python"]);
    }

    /// Every fixture plans without panicking, including the ones this slice
    /// cannot yet run. A graph the engine does not understand is reported, not
    /// crashed on.
    #[test]
    fn every_fixture_plans() {
        for name in [
            "customer-triage",
            "inbox-triage",
            "door-watch",
            "home-assistant",
        ] {
            let plan = plan(&fixture(name));
            assert!(
                !plan.order.is_empty(),
                "{name} planned no steps at all: {:?}",
                plan.problems
            );
        }
    }

    /// A live graph runs the part of itself downstream of what just happened,
    /// not the whole thing. Two sources on one canvas are two programs.
    #[test]
    fn an_event_sets_going_only_what_is_below_it() {
        let plan = plan(&fixture("inbox-triage"));

        // A file arriving runs the triage branch and nothing else.
        let from_folder = plan.downstream_of("watch", "file");
        assert!(
            from_folder.contains(&"classify".to_owned()),
            "{from_folder:?}"
        );
        assert!(from_folder.contains(&"branch".to_owned()));
        assert!(
            !from_folder.contains(&"digest".to_owned()),
            "a file must not fire the quarter-hourly digest: {from_folder:?}"
        );

        // The schedule runs the digest and nothing else. Its `tick` is an exec
        // port carrying no value, and it still sets the branch below it going.
        let from_clock = plan.downstream_of("schedule", "tick");
        assert!(from_clock.contains(&"digest".to_owned()), "{from_clock:?}");
        assert!(from_clock.contains(&"notify-email".to_owned()));
        assert!(!from_clock.contains(&"classify".to_owned()));
    }

    /// Every source the graph holds, so the panel can list what is armed.
    #[test]
    fn the_plan_knows_which_blocks_are_sources() {
        assert_eq!(
            plan(&fixture("inbox-triage")).sources(),
            ["webhook", "watch", "schedule"]
        );
        // The triage example has none, which is why it finishes.
        assert!(plan(&fixture("customer-triage")).sources().is_empty());
    }

    /// A loop frame is named as something this slice does not do, rather than
    /// quietly running its blocks once as though that were the same thing.
    #[test]
    fn a_loop_frame_says_it_will_not_repeat_yet() {
        let plan = plan(&fixture("inbox-triage"));
        assert!(
            plan.problems.iter().any(|p| p.contains("loop frame")),
            "{:?}",
            plan.problems
        );
    }
}
