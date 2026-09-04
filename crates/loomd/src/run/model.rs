//! Talking to a chat model.
//!
//! The provider is a trait with more than one implementation on purpose. One is
//! Ollama over HTTP, which is what a graph actually runs against. One is a
//! script, which is what the tests run against, and it exists for a reason
//! worth stating: a test that needs a language model to be installed, warmed
//! and in a particular mood is not a test. The tool-calling loop, the streaming,
//! the usage accounting and the event stream are all engine behaviour, and all
//! of it can be pinned down exactly if the model's side of the conversation is
//! written down in advance.
//!
//! What a scripted provider cannot tell you is whether a real model, given
//! these tool definitions, chooses to call them. That is a question about the
//! model rather than about the engine, and it is answered by running the thing.

use serde::{Deserialize, Serialize};
use std::fmt;

/// One turn in the conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    System {
        content: String,
    },
    User {
        content: String,
    },
    Assistant {
        content: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tool_calls: Vec<ToolCall>,
    },
    /// What a tool returned, fed back so the model can read it.
    Tool {
        name: String,
        content: String,
    },
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Message::User {
            content: content.into(),
        }
    }
    pub fn system(content: impl Into<String>) -> Self {
        Message::System {
            content: content.into(),
        }
    }
}

/// A tool the model may call, as the model sees it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDef {
    /// The name on the wire. See `wire_name`: not the same as what the console
    /// shows.
    pub name: String,
    pub description: String,
    /// A JSON Schema object describing the arguments.
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// The name a tool is given on the wire.
///
/// SPEC §6.3 says a Terminal is "offered to a model as `terminal.run`", and
/// that is the name the console and the trace show. It is not the name sent to
/// the model: the OpenAI function-calling schema, which Ollama follows, limits
/// a name to letters, digits, underscore and dash, so a dot would be rejected
/// or silently mangled by the very models this has to work with. The dot
/// becomes an underscore on the wire and comes back as a dot on the way in.
///
/// A block id may contain a dash already, which is why the split is on the last
/// underscore rather than the first: `watch-folder_read` is `watch-folder.read`.
pub fn wire_name(display: &str) -> String {
    display.replace('.', "_")
}

/// Turn a wire name back into the name a person reads.
pub fn display_name(wire: &str) -> String {
    match wire.rsplit_once('_') {
        Some((block, verb)) => format!("{block}.{verb}"),
        None => wire.to_owned(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDef>,
    /// The same width as the setting in the file. Narrowing to `f32` would
    /// send a model `0.699999988079071` for a user's `0.7`, which is harmless
    /// to a sampler and wrong in every log that shows it.
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u32>,
}

/// What one call to the model produced.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChatTurn {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Usage {
    pub tokens_in: u32,
    pub tokens_out: u32,
    /// Tokens per second over the response.
    pub rate: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModelError {
    /// The provider could not be reached at all. Its own sentence, because
    /// "connection refused" is not something to show a person.
    Unreachable(String),
    /// It answered, and said no.
    Refused(String),
    /// It answered with something this does not understand.
    Malformed(String),
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::Unreachable(s) | ModelError::Refused(s) | ModelError::Malformed(s) => {
                f.write_str(s)
            }
        }
    }
}

impl std::error::Error for ModelError {}

/// Somewhere to send text as it arrives, so a long answer is visible while it
/// is being written rather than after (SPEC §5.3).
pub type Sink<'a> = &'a mut dyn FnMut(&str);

pub trait ModelProvider: Send + Sync {
    /// What to call it in the status bar.
    fn name(&self) -> &str;

    /// Whether it runs on this machine. Drives both the "local · no charge"
    /// line in the Run panel (SPEC §8.5) and the warning before the first send
    /// to a remote service (SPEC §12.2).
    fn local(&self) -> bool;

    fn chat(&self, request: &ChatRequest, sink: Sink<'_>) -> Result<ChatTurn, ModelError>;
}

/// A provider that reads its side of the conversation from a script.
///
/// Turns are handed out in order. Running out is an error rather than a silent
/// empty answer: a loop that asked for one more turn than the script expected
/// is a bug in the loop, and it should say so.
pub struct Scripted {
    turns: std::sync::Mutex<std::collections::VecDeque<ChatTurn>>,
    /// Every request it was given, so a test can assert what the model was
    /// actually told — which tools it was offered, what the prompt became.
    pub seen: std::sync::Mutex<Vec<ChatRequest>>,
}

impl Scripted {
    pub fn new(turns: impl IntoIterator<Item = ChatTurn>) -> Self {
        Self {
            turns: std::sync::Mutex::new(turns.into_iter().collect()),
            seen: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// A turn that answers in words.
    pub fn says(text: &str) -> ChatTurn {
        ChatTurn {
            text: text.to_owned(),
            tool_calls: Vec::new(),
            usage: Usage {
                tokens_in: 0,
                tokens_out: text.split_whitespace().count() as u32,
                rate: 0.0,
            },
        }
    }

    /// A turn that calls a tool.
    pub fn calls(name: &str, arguments: serde_json::Value) -> ChatTurn {
        ChatTurn {
            text: String::new(),
            tool_calls: vec![ToolCall {
                name: name.to_owned(),
                arguments,
            }],
            usage: Usage::default(),
        }
    }
}

impl ModelProvider for Scripted {
    fn name(&self) -> &str {
        "scripted"
    }

    fn local(&self) -> bool {
        true
    }

    fn chat(&self, request: &ChatRequest, sink: Sink<'_>) -> Result<ChatTurn, ModelError> {
        self.seen.lock().unwrap().push(request.clone());
        let turn = self
            .turns
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| ModelError::Refused("the script has no more turns".into()))?;
        // Streamed a word at a time, so the sink is exercised the way a real
        // provider exercises it rather than being handed the answer whole.
        for (i, word) in turn.text.split_inclusive(' ').enumerate() {
            let _ = i;
            sink(word);
        }
        Ok(turn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dot_survives_the_round_trip_through_the_wire() {
        assert_eq!(wire_name("terminal.run"), "terminal_run");
        assert_eq!(display_name("terminal_run"), "terminal.run");
        // A block id with a dash in it keeps the dash and still splits right.
        assert_eq!(wire_name("watch-folder.read"), "watch-folder_read");
        assert_eq!(display_name("watch-folder_read"), "watch-folder.read");
    }

    /// A block id with an underscore is the case the split gets wrong, and it
    /// is worth being honest that it does: the last underscore is the verb's,
    /// so `my_terminal.run` comes back as `my_terminal.run` only because the
    /// split is from the right.
    #[test]
    fn the_split_is_from_the_right_so_an_underscore_in_the_id_survives() {
        assert_eq!(
            display_name(&wire_name("my_terminal.run")),
            "my_terminal.run"
        );
    }

    #[test]
    fn a_scripted_provider_streams_what_it_says() {
        let provider = Scripted::new([Scripted::says("the build fails at the link step")]);
        let mut streamed = String::new();
        let turn = provider
            .chat(
                &ChatRequest {
                    model: "test".into(),
                    messages: vec![Message::user("why?")],
                    tools: vec![],
                    temperature: None,
                    top_p: None,
                    max_tokens: None,
                },
                &mut |chunk| streamed.push_str(chunk),
            )
            .unwrap();
        assert_eq!(streamed, turn.text);
        assert_eq!(provider.seen.lock().unwrap().len(), 1);
    }

    /// Asking for one turn more than the script has is a bug in the caller, and
    /// it is reported rather than answered with silence.
    #[test]
    fn running_out_of_script_is_an_error() {
        let provider = Scripted::new([]);
        let err = provider
            .chat(
                &ChatRequest {
                    model: "test".into(),
                    messages: vec![],
                    tools: vec![],
                    temperature: None,
                    top_p: None,
                    max_tokens: None,
                },
                &mut |_| {},
            )
            .unwrap_err();
        assert!(matches!(err, ModelError::Refused(_)));
    }
}
