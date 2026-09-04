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

/// How many times a model may call tools before answering.
///
/// A model that keeps calling and never concludes is a real failure mode, and
/// an unbounded loop would run a graph forever while looking busy. Eight is
/// enough for the conversations these graphs describe — the triage example
/// needs one — and hitting it is reported rather than hidden.
const MAX_TOOL_ROUNDS: usize = 8;

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
}

struct State<'a> {
    emit: &'a mut dyn FnMut(RunEvent),
    ask: &'a mut dyn FnMut(&Warning) -> Decision,
    values: HashMap<Endpoint, Value>,
    /// Blocks that errored or were skipped, so downstream knows why it has no
    /// input to work from.
    failed: HashSet<String>,
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
            } else if plan.is_capability(&block.id) {
                BlockState::Ready
            } else if order.contains(&block.id) {
                BlockState::Queued
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
        let mut out = Outputs::new();
        out.insert("stdout".into(), Value::Text(output.stdout.clone()));
        let figure = output.figure();
        if !output.ok() {
            self.console(
                st,
                Some(&block.id),
                Level::Warn,
                format!("`{command}` exited {}", output.code),
            );
        }
        Ok((out, Some(figure)))
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
        messages.push(Message::user(prompt));

        // The tools this model holds, seen through any Toolbox between them.
        let held = plan.bindings.get(&block.id).cloned().unwrap_or_default();
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
                let result = self.dispatch(block, call, &held, st);
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
    fn dispatch(
        &self,
        caller: &Block,
        call: &ToolCall,
        held: &[String],
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

        let outcome: Result<blocks::Output, String> = match (callee.kind.as_str(), verb) {
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
            (kind, verb) => Err(format!("`{kind}` has no tool called `{verb}`")),
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
