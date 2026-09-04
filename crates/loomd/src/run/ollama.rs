//! The Ollama provider: a chat model running on this machine.
//!
//! Ollama's `/api/chat` answers with newline-delimited JSON, one object per
//! token, which is the same shape as the protocol this engine speaks to the
//! shell — so a token becomes an event without ever being buffered into a whole
//! answer first. That is what makes a long reply visible while it is being
//! written (SPEC §5.3).
//!
//! # What is proven here and what is not
//!
//! The tests below run against a stub server that speaks Ollama's wire format,
//! so the request shape, the streaming, the tool-call parsing and the usage
//! arithmetic are all covered. What they cannot cover is whether a real model,
//! handed these tool definitions, decides to call them: that is a question
//! about the model, not about this code, and it is answered by running it
//! against a real Ollama.

use super::model::{ChatRequest, ChatTurn, ModelError, ModelProvider, Sink, ToolCall};
use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::time::Duration;

/// Where Ollama listens unless told otherwise.
pub const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:11434";

pub struct Ollama {
    endpoint: String,
    agent: ureq::Agent,
}

impl Ollama {
    pub fn new(endpoint: Option<&str>) -> Self {
        let endpoint = endpoint
            .map(str::trim)
            .filter(|e| !e.is_empty())
            .unwrap_or(DEFAULT_ENDPOINT)
            .trim_end_matches('/')
            .to_owned();
        Self {
            endpoint,
            // No read timeout: a model thinking about a long prompt can be
            // quiet for a while before the first token, and cutting it off
            // would look like a crash. The run is stopped by the user, not by
            // a clock the user cannot see.
            agent: ureq::Agent::config_builder()
                .timeout_connect(Some(Duration::from_secs(5)))
                .build()
                .into(),
        }
    }

    /// Whether an Ollama is answering, and which models it holds.
    pub fn models(&self) -> Result<Vec<String>, ModelError> {
        let body: TagsBody = self
            .agent
            .get(format!("{}/api/tags", self.endpoint))
            .call()
            .map_err(|e| ModelError::Unreachable(unreachable_message(&self.endpoint, &e)))?
            .body_mut()
            .read_json()
            .map_err(|e| ModelError::Malformed(e.to_string()))?;
        Ok(body.models.into_iter().map(|m| m.name).collect())
    }
}

fn unreachable_message(endpoint: &str, error: &ureq::Error) -> String {
    // "connection refused" is not a sentence to show a person, and the fix is
    // always the same, so the message names it.
    match error {
        ureq::Error::ConnectionFailed | ureq::Error::Io(_) => {
            format!("no Ollama at {endpoint} — is `ollama serve` running?")
        }
        other => format!("{endpoint}: {other}"),
    }
}

impl ModelProvider for Ollama {
    fn name(&self) -> &str {
        "ollama"
    }

    /// True unless the endpoint was pointed somewhere else. A local model is
    /// free and needs no warning; one reached over a network is neither
    /// (SPEC §12.2, §15.4).
    fn local(&self) -> bool {
        self.endpoint.contains("127.0.0.1")
            || self.endpoint.contains("localhost")
            || self.endpoint.contains("[::1]")
    }

    fn chat(&self, request: &ChatRequest, sink: Sink<'_>) -> Result<ChatTurn, ModelError> {
        let mut options = serde_json::Map::new();
        if let Some(t) = request.temperature {
            options.insert("temperature".into(), serde_json::json!(t));
        }
        if let Some(p) = request.top_p {
            options.insert("top_p".into(), serde_json::json!(p));
        }
        if let Some(n) = request.max_tokens {
            options.insert("num_predict".into(), serde_json::json!(n));
        }

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "stream": true,
        });
        if !options.is_empty() {
            body["options"] = serde_json::Value::Object(options);
        }
        if !request.tools.is_empty() {
            // Ollama follows the OpenAI function-calling shape.
            body["tools"] = serde_json::Value::Array(
                request
                    .tools
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "type": "function",
                            "function": {
                                "name": t.name,
                                "description": t.description,
                                "parameters": t.parameters,
                            }
                        })
                    })
                    .collect(),
            );
        }

        let response = self
            .agent
            .post(format!("{}/api/chat", self.endpoint))
            .send_json(&body)
            .map_err(|e| match &e {
                ureq::Error::StatusCode(code) => ModelError::Refused(format!(
                    "ollama answered {code} — is `{}` pulled?",
                    request.model
                )),
                _ => ModelError::Unreachable(unreachable_message(&self.endpoint, &e)),
            })?;

        let mut turn = ChatTurn::default();
        let mut ns_total = 0u64;
        for line in BufReader::new(response.into_body().into_reader()).lines() {
            let line = line.map_err(|e| ModelError::Unreachable(e.to_string()))?;
            if line.trim().is_empty() {
                continue;
            }
            let chunk: ChatChunk = serde_json::from_str(&line)
                .map_err(|e| ModelError::Malformed(format!("{e}: {line}")))?;
            if let Some(error) = chunk.error {
                return Err(ModelError::Refused(error));
            }
            if let Some(message) = chunk.message {
                if !message.content.is_empty() {
                    turn.text.push_str(&message.content);
                    sink(&message.content);
                }
                for call in message.tool_calls {
                    turn.tool_calls.push(ToolCall {
                        name: call.function.name,
                        arguments: call.function.arguments,
                    });
                }
            }
            if chunk.done {
                turn.usage.tokens_in = chunk.prompt_eval_count.unwrap_or(0);
                turn.usage.tokens_out = chunk.eval_count.unwrap_or(0);
                ns_total = chunk.eval_duration.unwrap_or(0);
            }
        }

        // Ollama reports durations in nanoseconds. A rate needs both halves, so
        // a response that reported neither leaves the rate at zero rather than
        // dividing by one.
        if ns_total > 0 && turn.usage.tokens_out > 0 {
            let seconds = ns_total as f64 / 1_000_000_000.0;
            turn.usage.rate = (f64::from(turn.usage.tokens_out) / seconds) as f32;
        }
        Ok(turn)
    }
}

// ------------------------------------------------------------- what it sends

#[derive(Debug, Deserialize)]
struct ChatChunk {
    #[serde(default)]
    message: Option<ChunkMessage>,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
    #[serde(default)]
    eval_duration: Option<u64>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChunkMessage {
    #[serde(default)]
    content: String,
    #[serde(default)]
    tool_calls: Vec<ChunkToolCall>,
}

#[derive(Debug, Deserialize)]
struct ChunkToolCall {
    function: ChunkFunction,
}

#[derive(Debug, Deserialize)]
struct ChunkFunction {
    name: String,
    /// Ollama sends an object. Some builds send the JSON as a string, which is
    /// what the OpenAI API does, so both are accepted rather than one being
    /// treated as malformed.
    #[serde(deserialize_with = "arguments")]
    arguments: serde_json::Value,
}

fn arguments<'de, D>(d: D) -> Result<serde_json::Value, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = serde_json::Value::deserialize(d)?;
    Ok(match raw {
        serde_json::Value::String(s) => serde_json::from_str(&s).unwrap_or(serde_json::Value::Null),
        other => other,
    })
}

#[derive(Debug, Deserialize)]
struct TagsBody {
    #[serde(default)]
    models: Vec<TagsModel>,
}

#[derive(Debug, Deserialize)]
struct TagsModel {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::super::model::{Message, ToolDef};
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;

    /// A server that speaks Ollama's wire format and hands back a canned
    /// stream. Enough to pin down everything on this side of the socket.
    fn stub(body: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            // Read the request head and its body, so the test can assert what
            // was sent as well as what came back.
            let mut buf = Vec::new();
            let mut byte = [0u8; 1];
            let mut length = 0usize;
            loop {
                use std::io::Read;
                if stream.read(&mut byte).unwrap() == 0 {
                    break;
                }
                buf.push(byte[0]);
                if buf.ends_with(b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&buf).to_lowercase();
                    for line in head.lines() {
                        if let Some(n) = line.strip_prefix("content-length:") {
                            length = n.trim().parse().unwrap_or(0);
                        }
                    }
                    break;
                }
            }
            let mut body_bytes = vec![0u8; length];
            if length > 0 {
                use std::io::Read;
                stream.read_exact(&mut body_bytes).unwrap();
            }
            let sent = String::from_utf8_lossy(&body_bytes).into_owned();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
            stream.flush().unwrap();
            sent
        });
        (format!("http://127.0.0.1:{port}"), handle)
    }

    fn request() -> ChatRequest {
        ChatRequest {
            model: "llama3.2:3b".into(),
            messages: vec![Message::user("why did the build fail?")],
            tools: vec![ToolDef {
                name: "terminal_run".into(),
                description: "Run a command.".into(),
                parameters: serde_json::json!({ "type": "object" }),
            }],
            temperature: Some(0.7),
            top_p: None,
            max_tokens: None,
        }
    }

    #[test]
    fn a_streamed_answer_arrives_token_by_token() {
        let (endpoint, server) = stub(
            "{\"message\":{\"content\":\"The \"},\"done\":false}\n\
             {\"message\":{\"content\":\"build \"},\"done\":false}\n\
             {\"message\":{\"content\":\"failed.\"},\"done\":false}\n\
             {\"message\":{\"content\":\"\"},\"done\":true,\"prompt_eval_count\":210,\
               \"eval_count\":3,\"eval_duration\":100000000}\n",
        );
        let mut chunks = Vec::new();
        let turn = Ollama::new(Some(&endpoint))
            .chat(&request(), &mut |c| chunks.push(c.to_owned()))
            .unwrap();

        assert_eq!(chunks, ["The ", "build ", "failed."]);
        assert_eq!(turn.text, "The build failed.");
        assert_eq!(turn.usage.tokens_in, 210);
        assert_eq!(turn.usage.tokens_out, 3);
        // 3 tokens in 0.1s.
        assert!((turn.usage.rate - 30.0).abs() < 0.01, "{}", turn.usage.rate);

        // And the request carried what the model needs to answer.
        let sent: serde_json::Value = serde_json::from_str(&server.join().unwrap()).unwrap();
        assert_eq!(sent["model"], "llama3.2:3b");
        assert_eq!(sent["stream"], true);
        assert_eq!(sent["options"]["temperature"], 0.7);
        assert!(sent["options"].get("top_p").is_none(), "unset stays unsent");
        assert_eq!(sent["tools"][0]["function"]["name"], "terminal_run");
        assert_eq!(sent["messages"][0]["role"], "user");
    }

    #[test]
    fn a_tool_call_comes_back_as_one() {
        let (endpoint, _server) = stub(
            "{\"message\":{\"content\":\"\",\"tool_calls\":[{\"function\":\
               {\"name\":\"terminal_run\",\"arguments\":{\"command\":\"cargo build\"}}}]},\
               \"done\":true}\n",
        );
        let turn = Ollama::new(Some(&endpoint))
            .chat(&request(), &mut |_| {})
            .unwrap();
        assert_eq!(turn.tool_calls.len(), 1);
        assert_eq!(turn.tool_calls[0].name, "terminal_run");
        assert_eq!(turn.tool_calls[0].arguments["command"], "cargo build");
    }

    /// Some builds send the arguments as a JSON string rather than an object,
    /// which is what the OpenAI API does. Both are the same call.
    #[test]
    fn arguments_sent_as_a_string_are_still_arguments() {
        let (endpoint, _server) = stub(
            "{\"message\":{\"content\":\"\",\"tool_calls\":[{\"function\":\
               {\"name\":\"terminal_run\",\"arguments\":\"{\\\"command\\\":\\\"ls\\\"}\"}}]},\
               \"done\":true}\n",
        );
        let turn = Ollama::new(Some(&endpoint))
            .chat(&request(), &mut |_| {})
            .unwrap();
        assert_eq!(turn.tool_calls[0].arguments["command"], "ls");
    }

    /// An error in the stream is the model's answer, not a parse failure.
    #[test]
    fn an_error_in_the_stream_is_reported_as_a_refusal() {
        let (endpoint, _server) = stub("{\"error\":\"model 'nope' not found\"}\n");
        let err = Ollama::new(Some(&endpoint))
            .chat(&request(), &mut |_| {})
            .unwrap_err();
        assert!(matches!(err, ModelError::Refused(m) if m.contains("not found")));
    }

    /// Nothing listening is the ordinary case on a machine where Ollama is not
    /// running, and it says what to do about it.
    #[test]
    fn a_missing_ollama_says_so_in_words() {
        // Port 1 is never an Ollama.
        let err = Ollama::new(Some("http://127.0.0.1:1"))
            .chat(&request(), &mut |_| {})
            .unwrap_err();
        let ModelError::Unreachable(message) = err else {
            panic!("expected unreachable, got {err:?}");
        };
        assert!(message.contains("ollama serve"), "{message}");
    }

    #[test]
    fn an_endpoint_off_this_machine_is_not_local() {
        assert!(Ollama::new(None).local());
        assert!(Ollama::new(Some("http://localhost:11434")).local());
        assert!(!Ollama::new(Some("https://models.example.com")).local());
    }
}
