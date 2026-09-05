//! What the engine says while a graph runs.
//!
//! Events carry no request id: they arrive whenever the engine has something to
//! say, and a shell that does not understand one ignores it (see `rpc`). That
//! is what lets the engine gain an event without breaking an older shell, so
//! every variant here is additive by construction.
//!
//! The events are the *only* record of a run. Nothing about a run is written to
//! the graph file, because a `.loom` file describes what to run and never a run
//! (`graph_format::model`). A shell that missed an event has missed it; it can
//! ask for the run's state, but it cannot replay the stream.

use super::value::Value;
use serde::{Deserialize, Serialize};
use specta::Type;

/// The state a block is in, which is what the canvas draws as its status dot
/// (SPEC §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum BlockState {
    /// Placed, never run.
    Idle,
    /// Waiting on an upstream block.
    Queued,
    /// Executing now; its wires animate.
    Running,
    /// Produced a value this run.
    Done,
    /// Threw. The console holds the trace.
    Error,
    /// Skipped, wires kept.
    Disabled,
    /// A capability rather than a step: it was bound to a holder and will run
    /// only if called.
    ///
    /// Not one of §3.2's seven, because §3.2 describes blocks that execute.
    /// A Terminal offered to a model through a Toolbox never "runs" on its own
    /// — it waits to be called — and showing it as *done* would claim it had
    /// produced a value when it had not been asked for one.
    Ready,
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum RunOutcome {
    Finished,
    /// The user pressed stop.
    Stopped,
    /// At least one block errored. The run still reached the end: a block that
    /// throws does not tear the graph down, it reports (SPEC §12.1).
    Failed,
}

/// Where a console line came from, so the shell can colour it by category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
// `rename_all` covers the variant names; `rename_all_fields` covers the fields
// inside them, which is not the same attribute and not the same default. Without
// the second, `tokens_in` would be the one snake_case name in a camelCase
// protocol — invisible in Rust and a papercut in every shell that reads it.
#[serde(
    tag = "event",
    content = "data",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RunEvent {
    #[serde(rename = "run.started")]
    Started {
        run: String,
        graph: String,
        /// Every block the plan will visit, in the order it will visit them.
        /// The shell draws the whole run as queued before anything happens,
        /// which is what makes progress legible rather than surprising.
        order: Vec<String>,
    },

    #[serde(rename = "block.state")]
    BlockState {
        run: String,
        block: String,
        state: BlockState,
    },

    /// A chunk of output as it is produced: model tokens, a line of stdout.
    /// Sent as it happens, which is what makes streaming visible rather than a
    /// value that appears all at once (SPEC §5.3).
    #[serde(rename = "block.output")]
    BlockOutput {
        run: String,
        block: String,
        port: String,
        chunk: String,
    },

    /// A block finished and its output ports hold these values.
    #[serde(rename = "block.done")]
    BlockDone {
        run: String,
        block: String,
        /// One entry per output port that produced something.
        outputs: Vec<PortValue>,
        ms: u32,
        /// The short line the block shows inline while the run is in flight:
        /// `exit 101`, `1.2k tokens · 41/s` (SPEC §3.2).
        figure: Option<String>,
    },

    #[serde(rename = "block.error")]
    BlockError {
        run: String,
        block: String,
        message: String,
        /// The full trace, for the console. The message is the one line.
        detail: Option<String>,
    },

    /// What a camera is looking at, small enough to send often.
    ///
    /// A data URI rather than a path: a path means nothing to a window, and
    /// the alternative — letting the shell read any file the engine can — is a
    /// larger door than a preview is worth. It is a *thumbnail*; a captured
    /// frame still travels as a path (`run::value::Media`).
    #[serde(rename = "block.preview")]
    BlockPreview {
        run: String,
        block: String,
        image: String,
    },

    /// A source is armed and what it is watching: `watching ~/inbox`,
    /// `listening on :8420/inbox`, `every 15m` (SPEC §8.2).
    ///
    /// Its own event rather than a console line the shell would have to parse.
    /// A chip on a block is structured information; reading it back out of a
    /// sentence written for a person is how the two drift apart.
    #[serde(rename = "source.armed")]
    SourceArmed {
        run: String,
        block: String,
        state: String,
    },

    /// Where a loop frame has got to: `3 / 7`, and the item it is on. The
    /// frame's own status line (SPEC §3.5).
    #[serde(rename = "frame.state")]
    FrameState {
        run: String,
        frame: String,
        /// How many items are finished.
        at: u32,
        of: u32,
        /// One line describing the current item, or none before it starts.
        item: Option<String>,
    },

    /// What the avatar's face is doing (SPEC §11.3).
    ///
    /// Intent from the model, timing from the wires: the expression is what a
    /// tool call or an `express` wire asked for, the envelope is the shape of
    /// the audio that is about to play, and the gaze is wherever the `look`
    /// port pointed. None of the three waits for the others, so they arrive in
    /// one event and the shell animates each on its own clock.
    #[serde(rename = "face")]
    Face {
        run: String,
        block: String,
        /// The rig's folder name, so the shell knows which states to draw.
        rig: String,
        expression: String,
        /// 0–1. A smile at 0.2 is a different face from a smile at 1.
        intensity: f64,
        /// Loudness over time, 0–255 per bucket, twelve buckets a second.
        /// Empty when there is nothing to say.
        mouth: Vec<u8>,
        /// Who or what it is looking at, in the words the `look` port used.
        gaze: Option<String>,
        /// Where that is, when it is known: a point in the frame, `0..1` from
        /// the top left. A name alone leaves this empty and the shell decides.
        gaze_at: Option<[f64; 2]>,
        /// A one-shot gesture (`nod`, `shake`) carried by this event and this
        /// event only; it is not part of the face's state.
        gesture: Option<String>,
        /// Asleep after the idle timeout (SPEC §11.4). A rig with a `sleepy`
        /// state wears it; one without dims.
        asleep: bool,
        /// The idle the shell should animate: a blink about this often, and
        /// this many breaths a minute (zero for none). From the rig, overridden
        /// by the block.
        blink_ms: u32,
        breathe_per_min: u32,
        /// The mood's colour, for the media that have a colour and not a face.
        colour: String,
    },

    /// The engine held the graph itself, or let it go again.
    ///
    /// Hold is normally the person's: they press it and the shell knows because
    /// it asked. This is the other direction — a hardware fault pauses the graph
    /// (SPEC §12.1) and the transport has to show held without having been the
    /// one to do it, or Resume is a button nobody knows to press.
    #[serde(rename = "held")]
    Held { run: String, held: bool },

    /// Something long is being fetched: a model pulled into Ollama, weights
    /// downloaded. One event per step, the last one `done`, with `error` set
    /// when it did not finish. The shell draws a bar from it (SPEC §15.13:
    /// downloads are explicit and visible).
    Progress {
        run: String,
        /// What is being fetched, as a person would name it: `llama3.2:3b`.
        what: String,
        /// Bytes, as a float because the schema crosses to TypeScript, which
        /// has no integer wide enough for a model's size.
        completed: f64,
        /// Zero until the server says how much there is.
        total: f64,
        /// The last word from the server: `pulling manifest`, `success`.
        status: String,
        done: bool,
        error: Option<String>,
    },

    /// A wire carried a value. The canvas animates it (SPEC §5.3).
    #[serde(rename = "wire.active")]
    WireActive { run: String, wire: String },

    #[serde(rename = "console")]
    Console {
        run: String,
        /// The block it came from, or none for the run itself.
        source: Option<String>,
        level: Level,
        message: String,
    },

    /// A tool call, which is the trace tab's unit: who called what, with what,
    /// and what came back.
    #[serde(rename = "tool.call")]
    ToolCall {
        run: String,
        /// The block holding the handle — the model that made the call.
        caller: String,
        /// The block that answers it.
        callee: String,
        name: String,
        /// Whatever the model sent. `unknown` on the TypeScript side, for the
        /// same reason as `run::Value::Data`.
        #[specta(type = specta_typescript::Unknown)]
        arguments: serde_json::Value,
    },

    #[serde(rename = "tool.result")]
    ToolResult {
        run: String,
        caller: String,
        callee: String,
        name: String,
        result: String,
        ok: bool,
        ms: u32,
    },

    /// The run has stopped and is waiting for a person.
    ///
    /// This is SPEC §12.1 made concrete: the engine describes the action and
    /// waits. It does not decide, and there is no variant here for refusing —
    /// the only replies are continue and stop, because the application may warn
    /// before a dangerous action and may not prevent one.
    #[serde(rename = "run.warning")]
    Warning {
        run: String,
        /// Answered with `run.continue`.
        id: String,
        block: String,
        /// What is about to happen, in words, already fit to show.
        action: String,
        /// Why it warranted a warning (SPEC §12.2).
        reason: String,
        /// Whether the prompt offers "don't warn again for this block".
        remember: bool,
    },

    /// Tokens in and out, and what they cost. Local models are free, and the
    /// panel says so rather than showing a zero (SPEC §8.5).
    #[serde(rename = "run.usage")]
    Usage {
        run: String,
        block: String,
        tokens_in: u32,
        tokens_out: u32,
        /// Tokens per second, over the whole response.
        rate: f32,
        local: bool,
    },

    #[serde(rename = "run.finished")]
    Finished {
        run: String,
        outcome: RunOutcome,
        ms: u32,
        /// Named outputs, in the order their blocks appear in the file. What a
        /// headless run would print.
        results: Vec<PortValue>,
    },
}

/// A value on a named port.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PortValue {
    pub port: String,
    pub value: Value,
}

impl RunEvent {
    /// The run this belongs to. Every event has one: a shell may have two
    /// graphs open and needs to know which canvas to draw on.
    pub fn run(&self) -> &str {
        match self {
            RunEvent::Started { run, .. }
            | RunEvent::BlockState { run, .. }
            | RunEvent::BlockOutput { run, .. }
            | RunEvent::BlockDone { run, .. }
            | RunEvent::BlockError { run, .. }
            | RunEvent::BlockPreview { run, .. }
            | RunEvent::SourceArmed { run, .. }
            | RunEvent::FrameState { run, .. }
            | RunEvent::Face { run, .. }
            | RunEvent::Held { run, .. }
            | RunEvent::Progress { run, .. }
            | RunEvent::WireActive { run, .. }
            | RunEvent::Console { run, .. }
            | RunEvent::ToolCall { run, .. }
            | RunEvent::ToolResult { run, .. }
            | RunEvent::Warning { run, .. }
            | RunEvent::Usage { run, .. }
            | RunEvent::Finished { run, .. } => run,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The wire format is what the shell reads, so its shape is part of the
    /// protocol rather than an implementation detail.
    #[test]
    fn an_event_is_a_tagged_object() {
        let json = serde_json::to_value(RunEvent::BlockState {
            run: "r1".into(),
            block: "llm".into(),
            state: BlockState::Running,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "event": "block.state",
                "data": { "run": "r1", "block": "llm", "state": "running" }
            })
        );
    }

    #[test]
    fn every_event_names_its_run() {
        let events = [
            RunEvent::Started {
                run: "r1".into(),
                graph: "g".into(),
                order: vec![],
            },
            RunEvent::WireActive {
                run: "r1".into(),
                wire: "w1".into(),
            },
            RunEvent::Finished {
                run: "r1".into(),
                outcome: RunOutcome::Finished,
                ms: 0,
                results: vec![],
            },
        ];
        for event in events {
            assert_eq!(event.run(), "r1");
        }
    }
}
