//! Executing a plan.
//!
//! One run, start to finish, on the calling thread. Everything it has to say it
//! says through `emit`, and everything it has to ask it asks through `ask`;
//! it opens no socket and knows nothing about one. That is what lets the whole
//! of it be tested against a scripted model and a recording sink, and it is why
//! the daemon's threading is somebody else's problem.
//!
//! # A block that throws does not stop the run
//!
//! SPEC §12.1 is a policy about the user's authority over their own tools, and
//! it has a scheduling consequence: an error is a value the run carries, not an
//! exception that unwinds it. A failing block is marked, its downstream is
//! skipped for want of an input, and every other branch of the graph still
//! runs. The run ends `Failed` rather than ending early.

use super::blocks::{self, Outputs};
use super::event::{BlockState, Level, PortValue, RunEvent, RunOutcome};
use super::memory::{Episode, Hub, Order};
use super::model::{
    ChatRequest, Message, ModelError, ModelProvider, ToolCall, display_name, wire_name,
};
use super::plan::{Plan, plan};
use super::value::Value;
use graph_format::{Block, Endpoint, Graph};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// Bytes, as a person reads them.
fn human(bytes: u32) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.0} kB", f64::from(bytes) / 1024.0)
    } else {
        format!("{:.1} MB", f64::from(bytes) / (1024.0 * 1024.0))
    }
}

/// How many times a model may call tools before answering.
///
/// A model that keeps calling and never concludes is a real failure mode, and
/// an unbounded loop would run a graph forever while looking busy. Eight is
/// enough for the conversations these graphs describe — the triage example
/// needs one — and hitting it is reported rather than hidden.
const MAX_TOOL_ROUNDS: usize = 8;

/// What embeds a memory and a question so the two can be compared.
///
/// One model for both ends, always: two texts embedded by different models are
/// two points in two different spaces, and the cosine between them is a number
/// that means nothing.
const EMBED_MODEL: &str = "nomic-embed-text";

/// What the run is about to do, and why it warranted asking (SPEC §12.2).
#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    pub block: String,
    pub action: String,
    pub reason: String,
    pub remember: bool,
}

/// The only two answers. There is no "refuse": the application may warn before
/// a dangerous action and may not prevent one, so the choice is between going
/// ahead and stopping the whole run — never between going ahead and being
/// overruled on this one step (SPEC §12.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Decision {
    Continue,
    /// Continue, and do not ask again for this block during this run.
    ContinueAlways,
    Stop,
}

/// How a run ended, for whoever started it.
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub outcome: RunOutcome,
    pub ms: u32,
    /// What the Output blocks were given, in file order.
    pub results: Vec<PortValue>,
    pub errors: Vec<String>,
    /// What every port held when the run ended. A Once run has no use for it;
    /// a live one carries it into the next event when the graph keeps state
    /// (SPEC §8.4).
    pub values: HashMap<Endpoint, Value>,
}

pub struct Runner<'a> {
    pub graph: &'a Graph,
    /// The workspace folder. Relative paths in settings resolve against it, so
    /// a graph moves between machines with its files.
    pub root: &'a Path,
    pub provider: &'a dyn ModelProvider,
    pub run: String,
    /// Set from outside to stop the run.
    ///
    /// Stopping is somebody else's decision — a person pressing the button, on
    /// another thread — so it cannot be a return value from anything in here.
    /// It is checked between steps and between tool rounds rather than in the
    /// middle of one: a command that has started is allowed to finish, because
    /// killing a shell halfway leaves the user's machine in a state the graph
    /// never described.
    pub cancel: Arc<AtomicBool>,
    /// Where frames and audio live while the run lasts (SPEC §12.3).
    pub scratch: Arc<super::sense::Scratch>,
    /// What reads a frame or a sound.
    pub eye: Arc<dyn super::perceive::Perception>,
    /// The stores this run has open (SPEC §6.5).
    pub vault: Arc<super::memory::Vault>,
    /// The devices this run has open, and what a fault has stopped.
    ///
    /// It belongs to the run rather than to one pass through the graph, because
    /// a fault raised while handling one event has to still be holding when the
    /// next arrives — a Toolbox that un-paused itself between events would be
    /// a reflex with no memory (SPEC §9.1).
    pub bench: Arc<Bench>,
}

/// The devices a run has open, and the Toolboxes a fault has stopped.
#[derive(Default)]
pub struct Bench {
    devices: std::sync::Mutex<HashMap<String, Arc<dyn super::actuate::Device>>>,
    aim: std::sync::Mutex<HashMap<String, super::actuate::Aim>>,
    /// Toolboxes not taking calls until a person resumes or `motor.home`
    /// succeeds (SPEC §9.1).
    stopped: std::sync::Mutex<HashSet<String>>,
    /// Set for a run whose devices are scripted rather than real, which is how
    /// a graph with motors in it runs on a machine that has none.
    pub scripted: bool,
}

impl Bench {
    /// A bench whose devices answer from a script.
    pub fn scripted() -> Self {
        Self {
            scripted: true,
            ..Self::default()
        }
    }

    /// The device this block is, opened the first time it is asked for.
    fn device(&self, block: &Block) -> Result<Arc<dyn super::actuate::Device>, String> {
        if let Some(already) = self.devices.lock().unwrap().get(&block.id) {
            return Ok(Arc::clone(already));
        }
        let made: Arc<dyn super::actuate::Device> = if self.scripted {
            Arc::new(super::actuate::Scripted::default())
        } else {
            let path = blocks::setting(block, "port")
                .ok_or("this block has no port set: give it something like /dev/ttyUSB0")?;
            let baud = blocks::number(block, "baud").unwrap_or(115_200.0) as u32;
            Arc::new(super::actuate::Serial::open(
                path,
                baud,
                std::time::Duration::from_millis(500),
            )?)
        };
        self.devices
            .lock()
            .unwrap()
            .insert(block.id.clone(), Arc::clone(&made));
        Ok(made)
    }

    fn aimed(&self, block: &str) -> super::actuate::Aim {
        self.aim
            .lock()
            .unwrap()
            .get(block)
            .copied()
            .unwrap_or_default()
    }

    fn now_aimed(&self, block: &str, aim: super::actuate::Aim) {
        self.aim.lock().unwrap().insert(block.to_owned(), aim);
    }

    /// Stop every Toolbox this block's `fault` is wired into.
    fn fault(&self, graph: &Graph, block: &str) -> Vec<String> {
        let stopped: Vec<String> = graph
            .wires
            .iter()
            .filter(|w| w.from.node == block && w.from.port == "fault" && w.to.port == "pause")
            .map(|w| w.to.node.clone())
            .collect();
        self.stopped.lock().unwrap().extend(stopped.iter().cloned());
        stopped
    }

    /// Let them take calls again.
    pub fn clear(&self) {
        self.stopped.lock().unwrap().clear();
    }

    /// The Toolboxes a fault has stopped, for the panel and for dispatch.
    pub fn holding(&self) -> Vec<String> {
        let mut out: Vec<String> = self.stopped.lock().unwrap().iter().cloned().collect();
        out.sort();
        out
    }
}

struct State<'a> {
    emit: &'a mut dyn FnMut(RunEvent),
    ask: &'a mut dyn FnMut(&Warning) -> Decision,
    values: HashMap<Endpoint, Value>,
    /// Blocks that errored or were skipped, so downstream knows why it has no
    /// input to work from.
    failed: HashSet<String>,
    /// Blocks that have taken their turn. Distinguishes "produced nothing on
    /// this port" from "has not run yet", which is the whole of how a Branch
    /// decides what happens next.
    ran: HashSet<String>,
    /// Blocks the user said not to ask about again.
    trusted: HashSet<String>,
    errors: Vec<String>,
    stopped: bool,
}

impl<'a> Runner<'a> {
    /// Run the graph to the end.
    pub fn execute(
        &self,
        emit: &mut dyn FnMut(RunEvent),
        ask: &mut dyn FnMut(&Warning) -> Decision,
    ) -> Summary {
        let plan = plan(self.graph);
        let order = plan.order.clone();
        self.walk(&plan, &order, HashMap::new(), true, emit, ask)
    }

    /// Run just these steps, starting from values that are already known.
    ///
    /// This is what a live event does: the plan says which part of the graph
    /// is below whatever fired, and the value that fired is seeded onto its
    /// own port before anything runs (SPEC §8.2).
    pub fn execute_steps(
        &self,
        steps: &[String],
        seeded: HashMap<Endpoint, Value>,
        emit: &mut dyn FnMut(RunEvent),
        ask: &mut dyn FnMut(&Warning) -> Decision,
    ) -> Summary {
        let plan = plan(self.graph);
        self.walk(&plan, steps, seeded, false, emit, ask)
    }

    fn walk(
        &self,
        plan: &Plan,
        order: &[String],
        seeded: HashMap<Endpoint, Value>,
        announce: bool,
        emit: &mut dyn FnMut(RunEvent),
        ask: &mut dyn FnMut(&Warning) -> Decision,
    ) -> Summary {
        let started = Instant::now();
        let mut st = State {
            emit,
            ask,
            values: seeded,
            failed: HashSet::new(),
            ran: HashSet::new(),
            trusted: HashSet::new(),
            errors: Vec::new(),
            stopped: false,
        };

        if announce {
            (st.emit)(RunEvent::Started {
                run: self.run.clone(),
                graph: self.graph.id.clone(),
                order: order.to_vec(),
            });
            for problem in &plan.problems {
                self.console(&mut st, None, Level::Warn, problem.clone());
            }
        }

        // Everything the plan will visit shows as queued before anything
        // happens, and everything it will not shows as what it is.
        for block in &self.graph.blocks {
            let state = if block.disabled {
                BlockState::Disabled
            } else if order.contains(&block.id) {
                BlockState::Queued
            } else if plan.is_capability(&block.id) {
                // Ready, not queued: it is bound and waiting to be called. A
                // block that is both — a Terminal that runs its command and
                // answers calls — is queued, because that is the thing about to
                // happen (SPEC §3.3).
                BlockState::Ready
            } else if announce {
                BlockState::Idle
            } else {
                // A live event says nothing about the branches it is not
                // running, so the canvas keeps showing what they last did.
                continue;
            };
            (st.emit)(RunEvent::BlockState {
                run: self.run.clone(),
                block: block.id.clone(),
                state,
            });
        }

        for id in order {
            if st.stopped || self.cancelled(&mut st) {
                break;
            }
            if let Some(frame) = self.graph.frames.iter().find(|f| &f.id == id) {
                self.run_frame(frame, plan, &mut st);
                continue;
            }
            let Some(block) = self.block(id) else {
                continue;
            };
            if block.disabled {
                continue;
            }
            self.step(block, plan, &mut st);
        }

        // The run's results are what reached the Output blocks, which is what a
        // headless run would print.
        let mut results = Vec::new();
        for block in self.graph.blocks.iter().filter(|b| b.kind == "output") {
            let name = blocks::setting(block, "name")
                .unwrap_or(&block.id)
                .to_owned();
            if let Some(value) = self.gather(block, &mut st).get("value") {
                results.push(PortValue {
                    port: name,
                    value: value.clone(),
                });
            }
        }

        let outcome = if st.stopped {
            RunOutcome::Stopped
        } else if st.errors.is_empty() {
            RunOutcome::Finished
        } else {
            RunOutcome::Failed
        };
        let ms = started.elapsed().as_millis() as u32;
        if announce {
            (st.emit)(RunEvent::Finished {
                run: self.run.clone(),
                outcome,
                ms,
                results: results.clone(),
            });
        }
        Summary {
            outcome,
            ms,
            results,
            errors: st.errors,
            values: st.values,
        }
    }

    // ------------------------------------------------------------ one block

    fn step(&self, block: &Block, plan: &Plan, st: &mut State<'_>) {
        if self.off_the_path(block, st) {
            (st.emit)(RunEvent::BlockState {
                run: self.run.clone(),
                block: block.id.clone(),
                state: BlockState::Idle,
            });
            return;
        }

        let inputs = self.gather(block, st);

        // A block whose upstream failed has nothing to work from. It is skipped
        // rather than run with a hole in its inputs, and the console says which
        // block it was waiting on rather than leaving a gap.
        if let Some(missing) = self.missing_input(block, st) {
            self.console(
                st,
                Some(&block.id),
                Level::Warn,
                format!("skipped: {missing} produced nothing this run"),
            );
            st.failed.insert(block.id.clone());
            (st.emit)(RunEvent::BlockState {
                run: self.run.clone(),
                block: block.id.clone(),
                state: BlockState::Idle,
            });
            return;
        }

        (st.emit)(RunEvent::BlockState {
            run: self.run.clone(),
            block: block.id.clone(),
            state: BlockState::Running,
        });
        let started = Instant::now();

        let produced = match block.kind.as_str() {
            "llm" => self.run_model(block, plan, &inputs, st),
            "terminal" => self.run_terminal(block, &inputs, st),
            "python" => self.run_python(block, &inputs, st),
            "custom" => self.run_custom(block, &inputs, st),
            "webcam" | "microphone" | "keyboard" | "display" | "speaker" => {
                self.run_device(block, &inputs, st)
            }
            "objectDetection" | "object-detection" | "face-recognition" | "speechToText"
            | "speech-to-text" | "textToSpeech" | "text-to-speech" | "classifier" | "affect"
            | "embedding" => self.run_perception(block, &inputs, st),
            _ => blocks::pure_step(block, &inputs).map(|o| (o, None)),
        };

        match produced {
            Err(message) => {
                st.errors.push(format!("{}: {message}", block.id));
                st.failed.insert(block.id.clone());
                (st.emit)(RunEvent::BlockError {
                    run: self.run.clone(),
                    block: block.id.clone(),
                    message: message.clone(),
                    detail: None,
                });
                (st.emit)(RunEvent::BlockState {
                    run: self.run.clone(),
                    block: block.id.clone(),
                    state: BlockState::Error,
                });
                self.console(st, Some(&block.id), Level::Error, message);
            }
            Ok((outputs, figure)) => {
                st.ran.insert(block.id.clone());
                let mut sent = Vec::new();
                for (port, value) in &outputs {
                    let end = Endpoint::new(&block.id, port);
                    st.values.insert(end.clone(), value.clone());
                    sent.push(PortValue {
                        port: port.clone(),
                        value: value.clone(),
                    });
                }
                (st.emit)(RunEvent::BlockDone {
                    run: self.run.clone(),
                    block: block.id.clone(),
                    outputs: sent,
                    ms: started.elapsed().as_millis() as u32,
                    figure,
                });
                (st.emit)(RunEvent::BlockState {
                    run: self.run.clone(),
                    block: block.id.clone(),
                    state: BlockState::Done,
                });
                // The wires leaving this block light up now they carry
                // something (SPEC §5.3).
                for wire in &self.graph.wires {
                    if wire.from.node == block.id && outputs.contains_key(&wire.from.port) {
                        (st.emit)(RunEvent::WireActive {
                            run: self.run.clone(),
                            wire: wire.id.clone(),
                        });
                    }
                }
            }
        }
    }

    /// Run a loop frame: the blocks inside it, once per item (SPEC §3.5).
    ///
    /// # Asking once for two hundred items
    ///
    /// A block inside a frame that warns would warn per item, and a prompt
    /// that appears two hundred times is not a prompt. So the frame asks
    /// *before* it iterates, once per block that warns, describing what is
    /// about to happen that many times. That keeps SPEC §12.1 — the user is
    /// told and decides — without turning a Continue into a job.
    fn run_frame(&self, frame: &graph_format::Frame, plan: &Plan, st: &mut State<'_>) {
        let started = Instant::now();
        let inside = plan.frames.get(&frame.id).cloned().unwrap_or_default();
        let items = self.items_for(frame, st);

        (st.emit)(RunEvent::FrameState {
            run: self.run.clone(),
            frame: frame.id.clone(),
            at: 0,
            of: items.len() as u32,
            item: None,
        });

        if items.is_empty() {
            self.console(
                st,
                Some(&frame.id),
                Level::Warn,
                "nothing to iterate: the frame's `items` port has no list".into(),
            );
            return;
        }

        // Ask now, once, for anything inside that would ask per item.
        for id in &inside {
            let Some(block) = self.block(id) else {
                continue;
            };
            if !blocks::flag(block, "warnBefore") && !blocks::flag(block, "warnBeforeMove") {
                continue;
            }
            let permitted = self.permitted(
                st,
                Warning {
                    block: id.clone(),
                    action: format!("Run `{id}` once for each of {} items", items.len()),
                    reason: "A block inside a loop runs once per item. This asks once for all                              of them rather than once each."
                        .into(),
                    remember: true,
                },
            );
            if !permitted {
                return;
            }
            st.trusted.insert(id.clone());
        }

        let parallel = frame.parallel.max(1) as usize;
        let mut results: Vec<Value> = Vec::new();
        let mut errors: Vec<serde_json::Value> = Vec::new();
        let mut done = 0u32;

        for batch in items.chunks(parallel) {
            if st.stopped || self.cancelled(st) {
                break;
            }
            // Items in a batch run at the same time; their events are handed on
            // afterwards in item order. Interleaving two items' output would
            // produce a console nobody can read, and the point of the
            // parallelism is the work, not the log.
            let outcomes = self.run_batch(frame, &inside, batch, st);
            for (item, outcome) in batch.iter().zip(outcomes) {
                done += 1;
                (st.emit)(RunEvent::FrameState {
                    run: self.run.clone(),
                    frame: frame.id.clone(),
                    at: done,
                    of: items.len() as u32,
                    item: Some(item.summary(40)),
                });
                match outcome {
                    Ok(value) => results.push(value),
                    Err(why) => {
                        errors.push(serde_json::json!({ "item": item.as_data(), "error": why }));
                        if !frame.continue_on_error {
                            st.errors.push(format!("{}: {why}", frame.id));
                            self.console(st, Some(&frame.id), Level::Error, why);
                            self.finish_frame(frame, results, errors, done, started, st);
                            return;
                        }
                        self.console(st, Some(&frame.id), Level::Warn, why);
                    }
                }
            }
        }

        self.finish_frame(frame, results, errors, done, started, st);
    }

    /// One batch of items, run at the same time.
    fn run_batch(
        &self,
        frame: &graph_format::Frame,
        inside: &[String],
        batch: &[Value],
        st: &mut State<'_>,
    ) -> Vec<Result<Value, String>> {
        let mut collected: Vec<(Vec<RunEvent>, Result<Value, String>)> = Vec::new();

        std::thread::scope(|scope| {
            let mut handles = Vec::new();
            for item in batch {
                let seeded = {
                    let mut values = st.values.clone();
                    values.insert(Endpoint::new(&frame.id, &frame.as_name), item.clone());
                    values
                };
                let trusted = st.trusted.clone();
                handles.push(scope.spawn(move || {
                    let mut events = Vec::new();
                    let summary = {
                        let mut inner = State {
                            emit: &mut |e| events.push(e),
                            // A warning inside a batch was already asked before
                            // the loop started; anything that still asks here
                            // is answered by continuing, because there is
                            // nobody on this thread to ask.
                            ask: &mut |_| Decision::Continue,
                            values: seeded,
                            failed: HashSet::new(),
                            ran: HashSet::new(),
                            trusted,
                            errors: Vec::new(),
                            stopped: false,
                        };
                        for id in inside {
                            let Some(block) = self.block(id) else {
                                continue;
                            };
                            if block.disabled {
                                continue;
                            }
                            self.step(block, &plan(self.graph), &mut inner);
                        }
                        (inner.errors, inner.values)
                    };
                    let (errors, values) = summary;
                    let produced = self.frame_result(frame, inside, &values);
                    let outcome = if errors.is_empty() {
                        Ok(produced)
                    } else {
                        Err(errors.join("; "))
                    };
                    (events, outcome)
                }));
            }
            for handle in handles {
                match handle.join() {
                    Ok(pair) => collected.push(pair),
                    // A panic inside one item is that item's failure, not the
                    // graph's: the rest of the batch still reports.
                    Err(_) => collected.push((Vec::new(), Err("the item panicked".into()))),
                }
            }
        });

        let mut outcomes = Vec::new();
        for (events, outcome) in collected {
            for event in events {
                (st.emit)(event);
            }
            outcomes.push(outcome);
        }
        outcomes
    }

    /// What one pass through the frame produced: the value on the last block's
    /// first output, which is what a loop over a region means by "the result".
    fn frame_result(
        &self,
        _frame: &graph_format::Frame,
        inside: &[String],
        values: &HashMap<Endpoint, Value>,
    ) -> Value {
        for id in inside.iter().rev() {
            if let Some((_, value)) = values.iter().find(|(end, _)| &end.node == id) {
                return value.clone();
            }
        }
        Value::Null
    }

    fn finish_frame(
        &self,
        frame: &graph_format::Frame,
        results: Vec<Value>,
        errors: Vec<serde_json::Value>,
        done: u32,
        started: Instant,
        st: &mut State<'_>,
    ) {
        let mut out = Outputs::new();
        out.insert(
            "results".into(),
            Value::Data(serde_json::Value::Array(
                results.iter().map(Value::as_data).collect(),
            )),
        );
        if !errors.is_empty() {
            out.insert(
                "errors".into(),
                Value::Data(serde_json::Value::Array(errors)),
            );
        }
        // `done` is exec: it says the loop finished, and carries nothing.
        out.insert("done".into(), Value::Null);

        for (port, value) in &out {
            st.values
                .insert(Endpoint::new(&frame.id, port), value.clone());
        }
        (st.emit)(RunEvent::BlockDone {
            run: self.run.clone(),
            block: frame.id.clone(),
            outputs: out
                .iter()
                .map(|(port, value)| PortValue {
                    port: port.clone(),
                    value: value.clone(),
                })
                .collect(),
            ms: started.elapsed().as_millis() as u32,
            figure: Some(format!("{done} item{}", if done == 1 { "" } else { "s" })),
        });
        for wire in &self.graph.wires {
            if wire.from.node == frame.id && out.contains_key(&wire.from.port) {
                (st.emit)(RunEvent::WireActive {
                    run: self.run.clone(),
                    wire: wire.id.clone(),
                });
            }
        }
    }

    /// The items a frame will iterate.
    ///
    /// A list is its elements. Anything else is one item: a folder source
    /// hands the frame one file at a time, and a frame that refused to iterate
    /// a single value would make the commonest live graph impossible to write.
    fn items_for(&self, frame: &graph_format::Frame, st: &State<'_>) -> Vec<Value> {
        // `over` names the port the frame iterates; a wire into `items` is the
        // same thing said the other way round.
        let mut found: Option<Value> = st.values.get(&frame.over).cloned();
        if found.is_none() {
            for wire in &self.graph.wires {
                if wire.to.node == frame.id
                    && wire.to.port == "items"
                    && let Some(value) = st.values.get(&wire.from)
                {
                    found = Some(value.clone());
                    break;
                }
            }
        }
        let Some(value) = found else {
            return Vec::new();
        };

        // Read through `as_data`, so a list written as text and a list that
        // arrived as data are the same list. A model asked for JSON returns a
        // string, and a frame that would not iterate it would need a Convert
        // in front of every loop.
        let items = match value.as_data() {
            serde_json::Value::Array(items) => items
                .into_iter()
                .map(|v| match v {
                    serde_json::Value::String(s) => Value::Text(s),
                    other => Value::Data(other),
                })
                .collect(),
            // Anything else is one item: a folder source hands the frame one
            // file at a time, and a frame that refused a single value would
            // make the commonest live graph impossible to write.
            _ => vec![value],
        };
        let max = frame.max.max(1) as usize;
        items.into_iter().take(max).collect()
    }

    /// The values on this block's wired inputs.
    ///
    /// An `exec` wire is skipped. Exec is control flow, never a value
    /// (SPEC §4.3): a Schedule's tick says *run*, and reading it as an input
    /// handed a Terminal an empty string where its command should have been —
    /// which ran, succeeded, and did nothing.
    fn gather(&self, block: &Block, st: &mut State<'_>) -> Outputs {
        let mut inputs = Outputs::new();
        for wire in &self.graph.wires {
            if wire.to.node != block.id {
                continue;
            }
            if self.wire_type(wire) == Some(graph_format::PortType::Exec) {
                continue;
            }
            if let Some(value) = st.values.get(&wire.from) {
                // Fan-in on one port: the last writer wins, and in a Once run
                // that is the one latest in the order. SPEC §4.2 leaves the
                // rule to the engine; this is the one that matches reading the
                // file top to bottom.
                inputs.insert(wire.to.port.clone(), value.clone());
            }
        }
        inputs
    }

    /// The upstream block that was supposed to feed this one and did not.
    fn missing_input(&self, block: &Block, st: &State<'_>) -> Option<String> {
        for wire in &self.graph.wires {
            if wire.to.node != block.id {
                continue;
            }
            if st.failed.contains(&wire.from.node) {
                return Some(wire.from.node.clone());
            }
        }
        None
    }

    /// Whether this block is on a path a Branch did not take.
    ///
    /// A Branch produces a value on exactly one of its two exec ports, so a
    /// block below the other one has an incoming exec wire from a block that
    /// has run and said nothing. That silence is the branch: skipping here is
    /// what makes a Branch choose rather than fork.
    ///
    /// Only when *every* incoming exec wire is like that. A block reachable
    /// two ways runs if either way was taken, which is what a Merge downstream
    /// of a Branch has to mean.
    fn off_the_path(&self, block: &Block, st: &State<'_>) -> bool {
        let mut execs = 0usize;
        let mut silent = 0usize;
        for wire in &self.graph.wires {
            if wire.to.node != block.id
                || self.wire_type(wire) != Some(graph_format::PortType::Exec)
            {
                continue;
            }
            execs += 1;
            if st.ran.contains(&wire.from.node) && !st.values.contains_key(&wire.from) {
                silent += 1;
            }
        }
        execs > 0 && execs == silent
    }

    fn block(&self, id: &str) -> Option<&Block> {
        self.graph.blocks.iter().find(|b| b.id == id)
    }

    /// What a wire carries, from the port it leaves.
    fn wire_type(&self, wire: &graph_format::Wire) -> Option<graph_format::PortType> {
        let block = self.block(&wire.from.node)?;
        if let Some(port) = block
            .ports
            .iter()
            .find(|p| p.name == wire.from.port && p.side == graph_format::Side::Out)
        {
            return Some(port.port_type);
        }
        block_kinds::kind(&block.kind)?
            .ports
            .iter()
            .find(|p| p.name == wire.from.port && p.side == graph_format::Side::Out)
            .map(|p| p.port_type)
    }

    /// Whether someone pressed stop. Recorded on the state the first time it is
    /// seen, so the run ends `Stopped` rather than merely ending.
    fn cancelled(&self, st: &mut State<'_>) -> bool {
        if self.cancel.load(Ordering::Relaxed) {
            st.stopped = true;
        }
        st.stopped
    }

    fn console(&self, st: &mut State<'_>, source: Option<&str>, level: Level, message: String) {
        (st.emit)(RunEvent::Console {
            run: self.run.clone(),
            source: source.map(str::to_owned),
            level,
            message,
        });
    }

    /// Ask, if this block warns and has not been trusted for the rest of the
    /// run. Returns false when the user stopped.
    fn permitted(&self, st: &mut State<'_>, warning: Warning) -> bool {
        if st.trusted.contains(&warning.block) {
            return true;
        }
        match (st.ask)(&warning) {
            Decision::Continue => true,
            Decision::ContinueAlways => {
                st.trusted.insert(warning.block.clone());
                true
            }
            Decision::Stop => {
                st.stopped = true;
                self.console(
                    st,
                    Some(&warning.block),
                    Level::Warn,
                    "stopped at a warning".into(),
                );
                false
            }
        }
    }

    // ------------------------------------------------------------- runtimes

    fn run_terminal(
        &self,
        block: &Block,
        inputs: &Outputs,
        st: &mut State<'_>,
    ) -> Result<(Outputs, Option<String>), String> {
        // A `text` input overrides the setting, which is what lets a model's
        // thoughts be piped into a terminal (SPEC §13.3).
        let command = inputs
            .get("text")
            .map(Value::as_text)
            .or_else(|| blocks::setting(block, "command").map(str::to_owned))
            .ok_or("no command: the block's Command setting is empty and nothing is wired to it")?;
        let output = self.shell_with_warning(block, &command, st)?;

        // A command that exits non-zero has failed, and a Terminal *step* that
        // failed is a block that threw (SPEC §3.2). This is deliberately not
        // how a Terminal behaves as a *tool*: there, exit 101 is a result the
        // model reads and reasons about, which is the whole of §13.1. The two
        // paths differ because the two things differ — a step is part of the
        // program and a tool call is a question somebody asked.
        if !output.ok() {
            let why = output
                .stderr
                .lines()
                .rev()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .map(|l| format!("`{command}` exited {}: {l}", output.code))
                .unwrap_or_else(|| format!("`{command}` exited {}", output.code));
            return Err(why);
        }

        let mut out = Outputs::new();
        out.insert("stdout".into(), Value::Text(output.stdout.clone()));
        Ok((out, Some(output.figure())))
    }

    /// Run a command, asking first if the block is set to warn.
    fn shell_with_warning(
        &self,
        block: &Block,
        command: &str,
        st: &mut State<'_>,
    ) -> Result<blocks::Output, String> {
        if blocks::flag(block, "warnBefore")
            && !self.permitted(
                st,
                Warning {
                    block: block.id.clone(),
                    action: format!("Run `{command}`"),
                    reason: "Running a shell command is a dangerous action.".into(),
                    remember: true,
                },
            )
        {
            return Err("stopped before running".into());
        }
        let cwd = match blocks::setting(block, "cwd") {
            Some(dir) if Path::new(dir).is_absolute() => Path::new(dir).to_path_buf(),
            Some(dir) => self.root.join(dir),
            None => self.root.to_path_buf(),
        };
        blocks::shell(command, &cwd, self.graph.execution.timeout_sec)
    }

    fn run_python(
        &self,
        block: &Block,
        inputs: &Outputs,
        st: &mut State<'_>,
    ) -> Result<(Outputs, Option<String>), String> {
        let _ = inputs;
        let source = blocks::setting(block, "source")
            .ok_or("no source: the block's Source setting is empty")?;
        let path = if Path::new(source).is_absolute() {
            Path::new(source).to_path_buf()
        } else {
            self.root.join(source)
        };
        let code = std::fs::read_to_string(&path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;
        let output = blocks::python(block, &code, self.root)?;
        if !output.ok() {
            self.console(
                st,
                Some(&block.id),
                Level::Warn,
                format!("{source} exited {}", output.code),
            );
        }
        let mut out = Outputs::new();
        out.insert("value".into(), Value::Text(output.stdout.clone()));
        Ok((out, Some(output.figure())))
    }

    /// Run a custom block: parse its signature, call the function it names.
    ///
    /// The interface is derived here rather than taken from the block's stored
    /// `ports`, because the code is the truth and the stored ports are a copy
    /// of what it said last time (SPEC §10.1). A file edited outside the shell
    /// runs as it is now, not as the graph remembers it.
    fn run_custom(
        &self,
        block: &Block,
        inputs: &Outputs,
        st: &mut State<'_>,
    ) -> Result<(Outputs, Option<String>), String> {
        let source = block.source.as_ref().ok_or("this block has no code")?;
        let code = match source.mode {
            graph_format::SourceMode::Inline => source
                .code
                .clone()
                .ok_or("the block is in inline mode but holds no code")?,
            graph_format::SourceMode::File => {
                let path = source
                    .path
                    .as_deref()
                    .ok_or("the block is in file mode but names no file")?;
                let full = if Path::new(path).is_absolute() {
                    Path::new(path).to_path_buf()
                } else {
                    self.root.join(path)
                };
                std::fs::read_to_string(&full)
                    .map_err(|e| format!("could not read {}: {e}", full.display()))?
            }
        };

        let interfaces = block_source::parse(source.language, &code).map_err(|e| e.to_string())?;
        // A file with several functions makes several blocks; this block is the
        // one whose name it carries.
        let wanted = block.title.as_deref();
        let interface = wanted
            .and_then(|name| interfaces.iter().find(|i| i.name == name))
            .or_else(|| interfaces.first())
            .ok_or("the file declares no function")?;

        let (outputs, ran) = super::custom::run(block, interface, inputs, self.root)?;
        // What the function printed is the user's, and belongs in the console
        // beside every other line the graph produced.
        for line in ran.stdout.lines().filter(|l| !l.trim().is_empty()) {
            self.console(st, Some(&block.id), Level::Info, line.to_owned());
        }
        for line in ran.stderr.lines().filter(|l| !l.trim().is_empty()) {
            self.console(st, Some(&block.id), Level::Warn, line.to_owned());
        }
        let figure = Some(format!("{} ms", ran.ms));
        Ok((outputs, figure))
    }

    /// A block that reaches the world: a camera, a microphone, a screen, a
    /// speaker (SPEC §6.4, §6.6).
    fn run_device(
        &self,
        block: &Block,
        inputs: &Outputs,
        st: &mut State<'_>,
    ) -> Result<(Outputs, Option<String>), String> {
        use super::sense::{self, Input, Kind};
        let mut out = Outputs::new();

        match block.kind.as_str() {
            "webcam" => {
                let device = blocks::setting(block, "device").unwrap_or("lavfi:testsrc");
                let into = self.scratch.file(&block.id, "png");
                let frame = sense::frame(
                    &Input::read(device, Kind::Video),
                    blocks::setting(block, "resolution"),
                    &into,
                )?;
                let frame = self.record_if_asked(block, frame, st)?;
                // What the camera is looking at, for the panel's Live section
                // (Figure 6). A preview that cannot be made is not an error —
                // the frame is fine and the graph carries on without a picture.
                if let Ok(image) = sense::preview(&frame) {
                    (st.emit)(RunEvent::BlockPreview {
                        run: self.run.clone(),
                        block: block.id.clone(),
                        image,
                    });
                }
                let figure = format!("{} · {}", frame.mime, human(frame.bytes));
                out.insert("frames".into(), Value::Image(frame));
                return Ok((out, Some(figure)));
            }
            "microphone" => {
                let device = blocks::setting(block, "device").unwrap_or("lavfi:sine");
                let into = self.scratch.file(&block.id, "wav");
                // A microphone with no length listens for a second: long
                // enough to hold a word, short enough that a live graph is not
                // waiting on it.
                let seconds = blocks::number(block, "seconds").unwrap_or(1.0);
                let heard = sense::audio(&Input::read(device, Kind::Audio), seconds, &into)?;
                let heard = self.record_if_asked(block, heard, st)?;
                let figure = format!("{seconds:.1}s · {}", human(heard.bytes));
                out.insert("audio".into(), Value::Audio(heard));
                return Ok((out, Some(figure)));
            }
            "keyboard" => {
                // A Keyboard is a source in a live graph; as a step it stands
                // for whatever was typed into its placeholder, which is what
                // makes a graph runnable while it is being built.
                let typed = blocks::setting(block, "placeholder").unwrap_or_default();
                out.insert("text".into(), Value::Text(typed.to_owned()));
                return Ok((out, None));
            }
            "display" => {
                // A screen is a later slice; what a Display can honestly do
                // now is put what it was given where a person will see it.
                let shown = inputs.get("text").map(Value::as_text).unwrap_or_default();
                self.console(st, Some(&block.id), Level::Info, shown.clone());
                return Ok((out, Some(shown.chars().take(40).collect())));
            }
            "speaker" => {
                let Some(Value::Audio(sound)) = inputs.get("audio") else {
                    return Err("nothing is wired to the speaker's audio port".into());
                };
                sense::play(sound, blocks::setting(block, "device"))?;
                return Ok((out, Some(human(sound.bytes))));
            }
            _ => {}
        }
        Err(format!(
            "`{}` is not a device this engine can reach",
            block.kind
        ))
    }

    /// Recording is what moves a capture somewhere durable (SPEC §12.3).
    ///
    /// The default is off, and off means the file stays in the run's folder
    /// and goes away with it. Nothing has to remember to delete anything.
    fn record_if_asked(
        &self,
        block: &Block,
        what: super::value::Media,
        st: &mut State<'_>,
    ) -> Result<super::value::Media, String> {
        if !blocks::flag(block, "store") {
            return Ok(what);
        }
        let into = self.root.join("recordings").join(&block.id);
        let kept = super::sense::keep(&what, &into)?;
        self.console(
            st,
            Some(&block.id),
            Level::Warn,
            format!("recorded to {}", kept.path),
        );
        Ok(kept)
    }

    /// A block that reads what a frame or a sound contains (SPEC §6.1).
    fn run_perception(
        &self,
        block: &Block,
        inputs: &Outputs,
        st: &mut State<'_>,
    ) -> Result<(Outputs, Option<String>), String> {
        use super::perceive::{Affect, detections_as_value};
        let mut out = Outputs::new();
        let model = blocks::setting(block, "model").unwrap_or("");

        let image = || match inputs.get("image") {
            Some(Value::Image(media)) => Ok(media.clone()),
            Some(other) => Err(format!(
                "the image port was given {}, which is not a frame",
                other.port_type().as_str()
            )),
            None => Err("nothing is wired to the image port".to_owned()),
        };
        let text = || {
            inputs
                .get("text")
                .map(Value::as_text)
                .ok_or_else(|| "nothing is wired to the text port".to_owned())
        };

        match block.kind.as_str() {
            "objectDetection" | "object-detection" => {
                let seen = self
                    .eye
                    .detect(&image()?, model)
                    .map_err(|e| e.to_string())?;
                let figure = if seen.is_empty() {
                    "nothing".to_owned()
                } else {
                    format!("{} · {}", seen.len(), seen[0].label)
                };
                out.insert("objects".into(), detections_as_value(&seen));
                Ok((out, Some(figure)))
            }
            "face-recognition" => {
                // Enrolment is off by default and this engine does not enrol:
                // §12.2 lists enrolling a new face as warranting a warning,
                // and a warning nobody can answer is not one.
                if blocks::flag(block, "enrolment") {
                    self.console(
                        st,
                        Some(&block.id),
                        Level::Warn,
                        "enrolment is on, and this engine cannot enrol yet".into(),
                    );
                }
                let threshold = blocks::number(block, "threshold").unwrap_or(0.6);
                let who = self
                    .eye
                    .recognise(&image()?, threshold)
                    .map_err(|e| e.to_string())?;
                let figure = who.name.clone().unwrap_or_else(|| "someone".into());
                out.insert(
                    "person".into(),
                    Value::Data(serde_json::to_value(&who).unwrap_or(serde_json::Value::Null)),
                );
                Ok((out, Some(figure)))
            }
            "speechToText" | "speech-to-text" => {
                let Some(Value::Audio(sound)) = inputs.get("audio") else {
                    return Err("nothing is wired to the audio port".into());
                };
                let heard = self
                    .eye
                    .transcribe(sound, model)
                    .map_err(|e| e.to_string())?;
                let figure = format!("{:.1}s", heard.seconds);
                out.insert("text".into(), Value::Text(heard.text));
                Ok((out, Some(figure)))
            }
            "textToSpeech" | "text-to-speech" => {
                let into = self.scratch.file(&block.id, "wav");
                let said = self
                    .eye
                    .speak(
                        &text()?,
                        blocks::setting(block, "voice").unwrap_or(""),
                        &into,
                    )
                    .map_err(|e| e.to_string())?;
                let figure = human(said.bytes);
                out.insert("audio".into(), Value::Audio(said));
                Ok((out, Some(figure)))
            }
            "classifier" => {
                let labels: Vec<String> = match block.settings.get("labels") {
                    Some(graph_format::Setting::List(items)) => items
                        .iter()
                        .map(|i| match i {
                            graph_format::Setting::String(s) => s.clone(),
                            other => format!("{other:?}"),
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                let (label, confidence) = self
                    .eye
                    .classify(&text()?, &labels)
                    .map_err(|e| e.to_string())?;
                out.insert(
                    "data".into(),
                    Value::Data(serde_json::json!({ "label": label, "confidence": confidence })),
                );
                Ok((out, Some(label)))
            }
            "affect" => {
                let mood: Affect = self.eye.affect(&text()?).map_err(|e| e.to_string())?;
                let figure = mood.expression().to_owned();
                out.insert(
                    "affect".into(),
                    Value::Data(serde_json::json!({
                        "valence": mood.valence,
                        "arousal": mood.arousal,
                        // The expression is carried, not inferred downstream:
                        // an Avatar should not have to know the thresholds
                        // (SPEC §11.2).
                        "express": mood.expression(),
                    })),
                );
                Ok((out, Some(figure)))
            }
            "embedding" => {
                let vector = self.eye.embed(&text()?, model).map_err(|e| e.to_string())?;
                let figure = format!("{} dims", vector.len());
                out.insert("data".into(), Value::Data(serde_json::json!(vector)));
                Ok((out, Some(figure)))
            }
            other => Err(format!("`{other}` is not a perception this engine has")),
        }
    }

    // ------------------------------------------------------------ the model

    fn run_model(
        &self,
        block: &Block,
        plan: &Plan,
        inputs: &Outputs,
        st: &mut State<'_>,
    ) -> Result<(Outputs, Option<String>), String> {
        let mut messages = Vec::new();
        if let Some(system) = blocks::setting(block, "systemPrompt") {
            messages.push(Message::system(system));
        }
        if let Some(context) = inputs.get("context") {
            messages.push(Message::system(format!("Context:\n{}", context.as_text())));
        }
        let prompt = inputs
            .get("prompt")
            .map(Value::as_text)
            .ok_or("nothing is wired to the prompt")?;

        // What this model holds, seen through any Toolbox between them.
        let held = plan.bindings.get(&block.id).cloned().unwrap_or_default();

        // What it already knows, before it is asked anything (SPEC §9.2). The
        // recall is about the prompt, so what comes back is what the question
        // is about rather than whatever happened most recently.
        if let Some(hub) = self.memory(&held, plan, st) {
            let about = self.eye.embed(&prompt, EMBED_MODEL).ok();
            let recalled = hub.recall(about.as_deref());
            if !recalled.is_empty() {
                self.console(
                    st,
                    Some(&block.id),
                    Level::Info,
                    format!("recalled {} from memory", plural(recalled.len(), "thing")),
                );
                let lines: Vec<String> = recalled.iter().map(Episode::line).collect();
                messages.push(Message::system(format!(
                    "What you remember:\n{}",
                    lines.join("\n")
                )));
            }
        }

        messages.push(Message::user(prompt));

        let mut tools = Vec::new();
        for id in &held {
            if let Some(callee) = self.block(id) {
                for mut def in blocks::tools_of(callee) {
                    def.name = wire_name(&def.name);
                    tools.push(def);
                }
            }
        }

        let model = blocks::setting(block, "model")
            .unwrap_or(&self.graph.defaults.model)
            .to_owned();
        let request = ChatRequest {
            model,
            messages,
            tools,
            temperature: blocks::number(block, "temperature"),
            top_p: blocks::number(block, "topP"),
            max_tokens: blocks::number(block, "maxTokens").map(|t| t as u32),
        };

        let mut request = request;
        let mut answer = String::new();
        let mut calls_made: Vec<serde_json::Value> = Vec::new();
        let mut usage = super::model::Usage::default();

        for round in 0..MAX_TOOL_ROUNDS {
            if st.stopped || self.cancelled(st) {
                return Err("stopped".into());
            }
            let mut streamed = String::new();
            let turn = {
                let run = self.run.clone();
                let id = block.id.clone();
                let emit = &mut *st.emit;
                self.provider
                    .chat(&request, &mut |chunk| {
                        streamed.push_str(chunk);
                        emit(RunEvent::BlockOutput {
                            run: run.clone(),
                            block: id.clone(),
                            port: "text".into(),
                            chunk: chunk.to_owned(),
                        });
                    })
                    .map_err(|e| match e {
                        ModelError::Unreachable(m) => {
                            format!("{} is not answering: {m}", self.provider.name())
                        }
                        other => other.to_string(),
                    })?
            };
            usage.tokens_in += turn.usage.tokens_in;
            usage.tokens_out += turn.usage.tokens_out;
            usage.rate = turn.usage.rate;
            (st.emit)(RunEvent::Usage {
                run: self.run.clone(),
                block: block.id.clone(),
                tokens_in: usage.tokens_in,
                tokens_out: usage.tokens_out,
                rate: usage.rate,
                local: self.provider.local(),
            });

            if turn.tool_calls.is_empty() {
                answer = turn.text;
                break;
            }

            // The model asked for something. Record its turn, run each call,
            // and hand the results back so it can read them.
            request.messages.push(Message::Assistant {
                content: turn.text.clone(),
                tool_calls: turn.tool_calls.clone(),
            });
            for call in &turn.tool_calls {
                calls_made.push(serde_json::json!({
                    "name": display_name(&call.name),
                    "arguments": call.arguments,
                }));
                let result = self.dispatch(block, call, &held, plan, st);
                request.messages.push(Message::Tool {
                    name: call.name.clone(),
                    content: result,
                });
                if st.stopped {
                    return Err("stopped".into());
                }
            }

            if round == MAX_TOOL_ROUNDS - 1 {
                self.console(
                    st,
                    Some(&block.id),
                    Level::Warn,
                    format!(
                        "still calling tools after {MAX_TOOL_ROUNDS} rounds; \
                         answering with what it has"
                    ),
                );
                answer = turn.text;
            }
        }

        let mut out = Outputs::new();
        out.insert("text".into(), Value::Text(answer));
        if !calls_made.is_empty() {
            out.insert(
                "calls".into(),
                Value::Data(serde_json::Value::Array(calls_made)),
            );
        }
        let figure = if usage.rate > 0.0 {
            Some(format!("{} tokens · {:.0}/s", usage.tokens_out, usage.rate))
        } else if usage.tokens_out > 0 {
            Some(format!("{} tokens", usage.tokens_out))
        } else {
            None
        };
        Ok((out, figure))
    }

    /// Run one tool call and return what the model should read.
    ///
    /// A failure here is a *result*, not an error: a command that exits 101 is
    /// exactly what the triage example is about, and the model is supposed to
    /// read it and reason. Only a call the engine cannot make at all comes back
    /// as an apology.
    /// The memory this block holds, assembled from its slots (SPEC §9.2).
    ///
    /// A hub bundles stores; a store wired straight into a model is a hub of
    /// one, which is the same arrangement as a runtime wired straight into
    /// `llm.tools` and right for the same reason (§9.1).
    fn memory(&self, held: &[String], plan: &Plan, st: &mut State<'_>) -> Option<Hub> {
        let mut stores = Vec::new();
        let mut settings_from = None;
        for id in held {
            let Some(block) = self.block(id) else {
                continue;
            };
            if !blocks::is_memory(&block.kind) {
                continue;
            }
            // A hub is one block standing for several: its slots are the
            // stores, and its settings are the recall rules for all of them.
            let behind = match block.kind.as_str() {
                "memory-hub" => {
                    settings_from = Some(block);
                    plan.slots.get(id).cloned().unwrap_or_default()
                }
                _ => vec![id.clone()],
            };
            for store_id in behind {
                let Some(store_block) = self.block(&store_id) else {
                    continue;
                };
                match self.vault.store(store_block) {
                    Ok(store) => stores.push((store_id, store)),
                    Err(e) => self.console(
                        st,
                        Some(&store_id),
                        Level::Error,
                        format!("this store could not be opened: {e}"),
                    ),
                }
            }
        }
        if stores.is_empty() {
            return None;
        }
        let hub = settings_from;
        Some(Hub {
            stores,
            order: hub
                .and_then(|b| blocks::setting(b, "recall"))
                .map(Order::read)
                .unwrap_or(Order::RecentFirst),
            max: hub
                .and_then(|b| blocks::number(b, "maxRecalled"))
                .unwrap_or(12.0)
                .max(1.0) as usize,
            cutoff: hub.and_then(|b| blocks::number(b, "cutoff")).unwrap_or(0.0),
        })
    }

    /// Move, or go home (SPEC §6.6, §4.4).
    ///
    /// Three shapes of feedback come out of one call: the reply goes back to
    /// the model, the position goes onto the `state` port, and a fault fires
    /// `fault`, which stops any Toolbox that port is wired into before the
    /// model has finished its next thought.
    fn motors(
        &self,
        block: &Block,
        verb: &str,
        aim: super::actuate::Aim,
        st: &mut State<'_>,
    ) -> Result<blocks::Output, String> {
        use super::actuate::{Aim, interpret, within};

        // §12.2: a physical action warrants a warning, and the toggle is the
        // user's own (§12.1). Going home is not a move into the unknown — it is
        // how a fault is cleared — so it does not stop to ask.
        if verb == "move"
            && blocks::flag(block, "warnBeforeMove")
            && !self.permitted(
                st,
                Warning {
                    block: block.id.clone(),
                    action: format!("Move the motors to {}", aim.line()),
                    reason: "Moving a motor is a physical action.".into(),
                    remember: true,
                },
            )
        {
            return Err("stopped before moving".into());
        }

        // A limit is the machine's own geometry, not a policy about the user.
        // Past it, nothing moves and the block faults — which is what a real
        // controller does at an end stop.
        if verb == "move"
            && let Err(why) = within(
                aim,
                blocks::number(block, "panLimit"),
                blocks::number(block, "tiltLimit"),
            )
        {
            self.raise(block, &why, st);
            return Err(why);
        }

        let line = match verb {
            "home" => "home".to_owned(),
            _ => format!("move {:.0} {:.0}", aim.pan, aim.tilt),
        };
        let said = self.bench.device(block)?.send(&line)?;
        let reply = interpret(&said, if verb == "home" { Aim::default() } else { aim });

        if let Some(state) = &reply.state {
            self.bench.now_aimed(&block.id, aim);
            self.telemetry(block, "state", state, st);
        }
        match &reply.fault {
            Some(why) => {
                self.raise(block, why, st);
                Err(reply.text)
            }
            None => {
                // "or a clearing call (`motor.home`) succeeds" (SPEC §9.1).
                if verb == "home" && !self.bench.holding().is_empty() {
                    self.bench.clear();
                    self.console(
                        st,
                        Some(&block.id),
                        Level::Info,
                        "home cleared the fault; tool calls are going through again".into(),
                    );
                }
                Ok(said_ok(reply.text))
            }
        }
    }

    /// A device said something about itself: put it on the port and light the
    /// wire (SPEC §4.4).
    fn telemetry(&self, block: &Block, port: &str, what: &str, st: &mut State<'_>) {
        (st.emit)(RunEvent::BlockOutput {
            run: self.run.clone(),
            block: block.id.clone(),
            port: port.to_owned(),
            chunk: what.to_owned(),
        });
        st.values
            .insert(Endpoint::new(&block.id, port), Value::Text(what.to_owned()));
        for wire in &self.graph.wires {
            if wire.from.node == block.id && wire.from.port == port {
                (st.emit)(RunEvent::WireActive {
                    run: self.run.clone(),
                    wire: wire.id.clone(),
                });
            }
        }
    }

    /// A fault: the console says so, the `fault` port fires, and every Toolbox
    /// it is wired into stops taking calls (SPEC §4.4, §9.1).
    fn raise(&self, block: &Block, why: &str, st: &mut State<'_>) {
        self.console(st, Some(&block.id), Level::Error, format!("fault: {why}"));
        self.telemetry(block, "fault", why, st);
        for toolbox in self.bench.fault(self.graph, &block.id) {
            self.console(
                st,
                Some(&toolbox),
                Level::Warn,
                format!(
                    "{toolbox} is holding: no tool calls until the fault clears \
                     or you resume."
                ),
            );
        }
    }

    /// Consolidate one hub by name, from outside a run.
    ///
    /// The live loop has a hub id and a timer; everything else consolidation
    /// needs — the stores, the model, a console to report on — belongs to a
    /// `Runner`, so this is the door between the two.
    pub fn consolidate_hub(
        &self,
        hub_id: &str,
        plan: &Plan,
        emit: &mut dyn FnMut(RunEvent),
        ask: &mut dyn FnMut(&Warning) -> Decision,
    ) -> usize {
        let Some(block) = self.block(hub_id) else {
            return 0;
        };
        let mut st = State {
            emit,
            ask,
            values: HashMap::new(),
            failed: HashSet::new(),
            ran: HashSet::new(),
            trusted: HashSet::new(),
            errors: Vec::new(),
            stopped: false,
        };
        let held = vec![hub_id.to_owned()];
        match self.memory(&held, plan, &mut st) {
            None => 0,
            Some(hub) => self.consolidate(block, &hub, &mut st),
        }
    }

    /// Carry what fell out of working memory into the long-term store,
    /// summarising on the way (SPEC §9.2).
    ///
    /// Summarising asks the model, because the specification says the
    /// orchestrator writes one line per episode and a line written by anything
    /// else would be a different promise. A model that will not answer is not
    /// a reason to lose the memory: the text goes across as it stands, and the
    /// console says the summary was skipped.
    fn consolidate(&self, hub_block: &Block, hub: &Hub, st: &mut State<'_>) -> usize {
        let summarise = blocks::flag(hub_block, "summarise");
        let one_line = |episode: &Episode| {
            if !summarise {
                return episode.text.clone();
            }
            let request = ChatRequest {
                model: self.graph.defaults.model.clone(),
                messages: vec![
                    Message::system(
                        "Rewrite what the user says as one short line, worth                          keeping. Answer with the line and nothing else.",
                    ),
                    Message::user(episode.text.clone()),
                ],
                tools: Vec::new(),
                temperature: None,
                top_p: None,
                max_tokens: Some(64),
            };
            match self.provider.chat(&request, &mut |_| {}) {
                Ok(turn) if !turn.text.trim().is_empty() => turn.text.trim().to_owned(),
                _ => episode.text.clone(),
            }
        };
        let moved = hub.consolidate(&one_line);
        if !moved.is_empty() {
            self.console(
                st,
                Some(&hub_block.id),
                Level::Info,
                format!(
                    "consolidated {} into long-term memory",
                    plural(moved.len(), "memory")
                ),
            );
        }
        moved.len()
    }

    fn dispatch(
        &self,
        caller: &Block,
        call: &ToolCall,
        held: &[String],
        plan: &Plan,
        st: &mut State<'_>,
    ) -> String {
        let display = display_name(&call.name);
        let Some((callee_id, verb)) = display
            .rsplit_once('.')
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
        else {
            return format!("there is no tool called `{display}`");
        };
        let (callee_id, verb) = (callee_id.as_str(), verb.as_str());

        if !held.contains(&callee_id.to_owned()) {
            return format!("`{display}` is not one of the tools you were given");
        }

        // A fault stopped the Toolbox this tool is behind (SPEC §9.1). It
        // pauses; it never locks — `motor.home` still goes through, because a
        // pause a person cannot get out of from inside the graph is a lock.
        //
        // The refusal is worked out here and reported below with every other
        // outcome, rather than returned from here: a call that was refused is
        // still a call the trace should show, and an early return would have
        // made the one call a person most wants to see the one that leaves no
        // record.
        let refused = (verb != "home")
            .then(|| {
                self.bench.holding().into_iter().find(|toolbox| {
                    plan.slots
                        .get(toolbox)
                        .is_some_and(|behind| behind.iter().any(|b| b == callee_id))
                })
            })
            .flatten();
        let Some(callee) = self.block(callee_id) else {
            return format!("there is no block called `{callee_id}`");
        };

        (st.emit)(RunEvent::ToolCall {
            run: self.run.clone(),
            caller: caller.id.clone(),
            callee: callee_id.to_owned(),
            name: display.clone(),
            arguments: call.arguments.clone(),
        });
        (st.emit)(RunEvent::BlockState {
            run: self.run.clone(),
            block: callee_id.to_owned(),
            state: BlockState::Running,
        });

        let started = Instant::now();
        let arg = |key: &str| {
            call.arguments
                .get(key)
                .and_then(|v| v.as_str())
                .map(str::to_owned)
        };

        let outcome: Result<blocks::Output, String> = if let Some(toolbox) = refused {
            Err(format!(
                "not called: a fault stopped {toolbox}. Call a clearing tool \
                 such as `motor.home`, or ask the user to resume."
            ))
        } else {
            match (callee.kind.as_str(), verb) {
                ("terminal", "run") => {
                    match arg("command")
                        .or_else(|| blocks::setting(callee, "command").map(str::to_owned))
                    {
                        None => Err("no command was given and the block has no default".into()),
                        Some(command) => self.shell_with_warning(callee, &command, st),
                    }
                }
                ("python", "exec") => match arg("code") {
                    None => Err("no code was given".into()),
                    Some(code) => blocks::python(callee, &code, self.root),
                },
                ("motors", verb @ ("move" | "home")) => {
                    let number = |key: &str| call.arguments.get(key).and_then(|v| v.as_f64());
                    let aim = match verb {
                        "home" => super::actuate::Aim::default(),
                        _ => super::actuate::Aim {
                            pan: number("pan").unwrap_or(self.bench.aimed(callee_id).pan),
                            tilt: number("tilt").unwrap_or(self.bench.aimed(callee_id).tilt),
                        },
                    };
                    self.motors(callee, verb, aim, st)
                }
                ("usb-device", "send") => match arg("line") {
                    None => Err("no line was given to send".into()),
                    Some(line) => self
                        .bench
                        .device(callee)
                        .and_then(|device| device.send(&line))
                        .map(said),
                },
                (kind, "remember") if blocks::is_memory(kind) => match arg("text") {
                    None => Err("nothing was given to remember".into()),
                    Some(text) => {
                        let sort = arg("kind").unwrap_or_else(|| "episode".into());
                        let vector = self.eye.embed(&text, EMBED_MODEL).ok();
                        match self.memory(held, plan, st) {
                            None => Err("this memory has no stores wired into it".into()),
                            Some(hub) => hub.remember(&text, &sort, vector.as_deref()).map(|id| {
                                // "when working memory is full" (SPEC §9.2): the
                                // moment something new arrives is the moment
                                // something old may have fallen out.
                                self.consolidate(callee, &hub, st);
                                said(format!("remembered, as {id}"))
                            }),
                        }
                    }
                },
                // §12.2 warrants a warning before deleting a person from long-term
                // memory. Forgetting is the only memory call that can lose
                // something, so it is the one that asks first — and, as everywhere
                // else, it warns and does not block (§12.1).
                (kind, "forget") if blocks::is_memory(kind) => match arg("what") {
                    None => Err("nothing was given to forget".into()),
                    Some(what) => {
                        let asked = self.permitted(
                            st,
                            Warning {
                                block: callee.id.clone(),
                                action: format!("Forget everything matching `{what}`"),
                                reason: "Deleting a person deletes every sighting of \
                                     them, from every store."
                                    .into(),
                                remember: false,
                            },
                        );
                        match asked {
                            false => Err("stopped before forgetting".into()),
                            true => match self.memory(held, plan, st) {
                                None => Err("this memory has no stores wired into it".into()),
                                Some(hub) => hub
                                    .forget(&what)
                                    .map(|gone| said(format!("forgot {}", plural(gone, "memory")))),
                            },
                        }
                    }
                },
                (kind, verb) => Err(format!("`{kind}` has no tool called `{verb}`")),
            }
        };

        let ms = started.elapsed().as_millis() as u32;
        let (result, ok) = match outcome {
            Ok(output) => {
                (st.emit)(RunEvent::BlockDone {
                    run: self.run.clone(),
                    block: callee_id.to_owned(),
                    outputs: vec![PortValue {
                        port: "stdout".into(),
                        value: Value::Text(output.stdout.clone()),
                    }],
                    ms,
                    figure: Some(output.figure()),
                });
                // A tool's own output port carries what it produced, so a wire
                // from it shows the same text the model read.
                st.values.insert(
                    Endpoint::new(callee_id, "stdout"),
                    Value::Text(output.stdout.clone()),
                );
                (output.as_tool_result(), output.ok())
            }
            Err(message) => {
                (st.emit)(RunEvent::BlockError {
                    run: self.run.clone(),
                    block: callee_id.to_owned(),
                    message: message.clone(),
                    detail: None,
                });
                (message, false)
            }
        };

        (st.emit)(RunEvent::ToolResult {
            run: self.run.clone(),
            caller: caller.id.clone(),
            callee: callee_id.to_owned(),
            name: display,
            result: result.clone(),
            ok,
            ms,
        });
        // Back to standing ready: a capability that answered one call may be
        // asked again, so it is not *done* (see `BlockState::Ready`).
        (st.emit)(RunEvent::BlockState {
            run: self.run.clone(),
            block: callee_id.to_owned(),
            state: BlockState::Ready,
        });
        result
    }
}

/// A tool result that is a sentence rather than a process.
///
/// Memory has no exit code and no stderr; what it has to say is one line, and
/// dressing it as a finished command would put `exit 0` in front of it.
fn said_ok(what: String) -> blocks::Output {
    said(what)
}

fn said(what: String) -> blocks::Output {
    blocks::Output {
        stdout: what,
        stderr: String::new(),
        code: 0,
        ms: 0,
    }
}

/// `1 memory`, `3 memories`, `no memories`.
fn plural(n: usize, thing: &str) -> String {
    let many = match thing {
        "memory" => "memories".to_owned(),
        other => format!("{other}s"),
    };
    match n {
        0 => format!("no {many}"),
        1 => format!("1 {thing}"),
        n => format!("{n} {many}"),
    }
}
