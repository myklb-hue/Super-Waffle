//! What travels along a wire while a graph runs.
//!
//! This is not `graph_format::Value`. That one is a *setting*: a scalar the
//! user typed into the inspector, which the file has to round-trip. This one is
//! a *value in flight*, and the difference matters in both directions — a
//! setting can never be an image, and a value in flight is never written to
//! disk. Keeping them apart means neither type has to carry the other's
//! compromises.
//!
//! There is one variant per flow type in SPEC §4.1, and none for the closed
//! types: `tools` and `memory` are handles and `exec` is control flow, so none
//! of the three is a value and none can appear here (§4.3). What a handle wire
//! carries is a binding, which the plan resolves before the run starts.

use graph_format::PortType;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Value {
    Text(String),
    Data(serde_json::Value),
    /// A path on disk. Held as a string rather than a `PathBuf` because it
    /// crosses the socket as JSON and the shell shows it verbatim.
    File(String),
    Image(Media),
    Audio(Media),
    /// A port that produced nothing. Distinct from a port that was never
    /// reached: the plan knows the difference and the console says which.
    Null,
}

/// Bytes that are too big to put in an event, named by where they landed.
///
/// An image or an audio buffer goes to a file in the run's scratch folder and
/// travels as a reference. The alternative is base64 in a JSON line, which
/// would make the console log unreadable and the socket the bottleneck.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    pub path: String,
    /// `image/png`, `audio/wav`. The shell picks a preview from it.
    pub mime: String,
    pub bytes: u32,
}

impl Value {
    /// The port type this value satisfies.
    pub fn port_type(&self) -> PortType {
        match self {
            Value::Text(_) => PortType::Text,
            Value::Data(_) => PortType::Data,
            Value::File(_) => PortType::File,
            Value::Image(_) => PortType::Image,
            Value::Audio(_) => PortType::Audio,
            Value::Null => PortType::Any,
        }
    }

    /// The value as text, for a port that wants text.
    ///
    /// Data is rendered as compact JSON rather than refused: a model's prompt
    /// takes text, and a record wired into one should arrive as the record,
    /// not as an error. This is the same coercion the grammar already allows
    /// (`data` is accepted by `text`, SPEC §4.1) made concrete.
    pub fn as_text(&self) -> String {
        match self {
            Value::Text(s) => s.clone(),
            Value::Data(v) => match v {
                // A JSON string arrives as its contents, not with its quotes.
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            },
            Value::File(p) => p.clone(),
            Value::Image(m) | Value::Audio(m) => m.path.clone(),
            Value::Null => String::new(),
        }
    }

    /// The value as JSON, for a port that wants data.
    pub fn as_data(&self) -> serde_json::Value {
        match self {
            Value::Data(v) => v.clone(),
            Value::Text(s) => {
                // Text that happens to be JSON arrives as the record it
                // describes. A model asked for JSON returns a string, and
                // wiring it into a `data` port should not need a Convert.
                serde_json::from_str(s).unwrap_or_else(|_| serde_json::Value::String(s.clone()))
            }
            Value::File(p) => serde_json::Value::String(p.clone()),
            Value::Image(m) | Value::Audio(m) => {
                serde_json::json!({ "path": m.path, "mime": m.mime, "bytes": m.bytes })
            }
            Value::Null => serde_json::Value::Null,
        }
    }

    /// One line, fit to show on the block while the run is in flight
    /// (SPEC §3.2's "live figures"). Never more than `width` characters, and
    /// never a newline, so the block's height does not change mid-run.
    pub fn summary(&self, width: usize) -> String {
        let whole = match self {
            Value::Data(v) => v.to_string(),
            Value::Image(m) => format!("{} · {}", m.mime, human_bytes(m.bytes)),
            Value::Audio(m) => format!("{} · {}", m.mime, human_bytes(m.bytes)),
            Value::Null => return "—".into(),
            other => other.as_text(),
        };
        let flat = whole.split_whitespace().collect::<Vec<_>>().join(" ");
        if flat.chars().count() <= width {
            return flat;
        }
        let kept: String = flat.chars().take(width.saturating_sub(1)).collect();
        format!("{kept}…")
    }
}

fn human_bytes(bytes: u32) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.0} kB", f64::from(bytes) / 1024.0)
    } else {
        format!("{:.1} MB", f64::from(bytes) / (1024.0 * 1024.0))
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_and_data_cross_without_a_convert() {
        // The grammar allows data into a text port, so the coercion has to
        // exist somewhere; this is where.
        let data = Value::Data(serde_json::json!({ "exit": 101 }));
        assert_eq!(data.as_text(), r#"{"exit":101}"#);
        // And text that is already JSON arrives as the record.
        let text = Value::Text(r#"{"label":"urgent"}"#.into());
        assert_eq!(text.as_data(), serde_json::json!({ "label": "urgent" }));
    }

    /// A JSON string is a string. Rendering it with its quotes would put
    /// `"hello"` into a prompt, which is not what the wire carries.
    #[test]
    fn a_json_string_loses_its_quotes() {
        assert_eq!(
            Value::Data(serde_json::json!("hello")).as_text(),
            "hello".to_owned()
        );
    }

    /// Text that is not JSON stays text rather than becoming an error.
    #[test]
    fn plain_text_survives_a_data_port() {
        assert_eq!(
            Value::Text("cargo build".into()).as_data(),
            serde_json::json!("cargo build")
        );
    }

    #[test]
    fn a_summary_is_one_short_line() {
        let long = Value::Text("error: linking with `cc` failed\n  = note: ld returned 1".into());
        let line = long.summary(30);
        assert_eq!(line.chars().count(), 30);
        assert!(!line.contains('\n'));
        assert!(line.ends_with('…'));
        // A short value is shown whole, without an ellipsis.
        assert_eq!(Value::Text("done".into()).summary(30), "done");
        assert_eq!(Value::Null.summary(30), "—");
    }

    #[test]
    fn media_says_how_big_it_is() {
        let image = Value::Image(Media {
            path: "/tmp/frame.png".into(),
            mime: "image/png".into(),
            bytes: 2_400_000,
        });
        assert_eq!(image.summary(40), "image/png · 2.3 MB");
        // The value itself is still the path, so a file port can take it.
        assert_eq!(image.as_text(), "/tmp/frame.png");
    }
}
