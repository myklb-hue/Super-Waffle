//! The signature is the block (SPEC §10.1).
//!
//! A custom block is a function, and its interface is read from its code
//! rather than declared beside it. A parameter without a default is an input
//! port; one with a default is a setting; the return annotation is the output.
//! That single rule is why a custom block cannot drift from what it does: there
//! is nowhere for a second description to live.
//!
//! # Why this does not use a real parser
//!
//! It re-parses on every save and on every editor blur (§10.3), against code
//! the user is in the middle of writing. What it needs is to find a signature
//! in a half-finished file and say something useful about it — not to be right
//! about Python. A grammar would refuse the file outright the moment a body
//! was incomplete, which is most of the time while someone is typing.
//!
//! So this reads signatures and nothing else. It never looks inside a function
//! body, which is also why it is the same shape for four languages: the part it
//! reads is the part they all spell similarly.

pub mod python;
pub mod reload;
pub mod shell;
pub mod typescript;

use graph_format::{Language, Port, PortType, Side};
use serde::{Deserialize, Serialize};
use specta::Type;

/// A block derived from one function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Interface {
    /// The function's own name, which becomes the block's title.
    pub name: String,
    /// The docstring, or the comment above the function.
    pub description: Option<String>,
    /// From `@block(icon=…)`. None leaves the block with the Custom shelf's
    /// default rather than guessing one from the name.
    pub icon: Option<String>,
    /// From `@block(category=…)`, which decides the shelf it lands on.
    pub category: Option<String>,
    pub ports: Vec<Port>,
    pub settings: Vec<Generated>,
    /// Where the function starts, 1-based. Used to point at the right line
    /// when something is wrong with it.
    pub line: u32,
}

/// A setting the code asked for, with the control it should draw.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Generated {
    pub name: String,
    /// Title-cased from the name: `max_items` becomes `Max items`.
    pub label: String,
    pub kind: Control,
    /// The default, exactly as it was written in the code. Editing the setting
    /// rewrites this text in place (§10.3), so it is kept verbatim rather than
    /// parsed and re-printed: `0.60` should not silently become `0.6`.
    pub default: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// A `Literal[...]` or a union of strings becomes a choice.
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Control {
    Text,
    Multiline,
    Number,
    Range,
    Bool,
    Select,
    Path,
}

/// What went wrong, and where.
///
/// A parse failure is shown on the block with its line number while the
/// previous interface stays, so the graph keeps running around it (§10.4).
/// That is only possible because this reports rather than throws.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SourceError {
    /// 1-based, so it matches what an editor's gutter says.
    pub line: u32,
    pub message: String,
}

impl SourceError {
    pub fn new(line: u32, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

/// Read every block in a file.
///
/// A file with several decorated functions makes several blocks, one per
/// function (§10.5).
pub fn parse(language: Language, source: &str) -> Result<Vec<Interface>, SourceError> {
    match language {
        Language::Python => python::parse(source),
        Language::Typescript | Language::Javascript => typescript::parse(source),
        Language::Shell => shell::parse(source),
    }
}

// ------------------------------------------------------------ shared pieces

/// An annotation as a port type (SPEC §10.1).
///
/// The names are the type system's own (§4.1), matched without regard to case
/// so `Image` and `image` are the same annotation. Anything else — a language's
/// own type, a class the user wrote, no annotation at all — is `any`, which is
/// the honest answer: the grammar does not know what it is, and `any` is the
/// type that says so.
pub fn port_type(annotation: Option<&str>) -> PortType {
    let Some(name) = annotation else {
        return PortType::Any;
    };
    // `Optional[Image]`, `Image | None` and `list[Image]` all carry their real
    // type inside; the outer wrapper says how many or whether, not what.
    let inner = unwrap_annotation(name);
    match inner.to_ascii_lowercase().as_str() {
        "text" | "str" | "string" => PortType::Text,
        "data" | "dict" | "record" | "json" | "object" => PortType::Data,
        "image" => PortType::Image,
        "audio" => PortType::Audio,
        "file" | "path" => PortType::File,
        "stream" | "iterator" | "asynciterator" => PortType::Stream,
        "tools" => PortType::Tools,
        "memory" => PortType::Memory,
        "exec" | "trigger" => PortType::Exec,
        _ => PortType::Any,
    }
}

/// The type inside a wrapper, if the annotation is one.
fn unwrap_annotation(name: &str) -> String {
    let name = name.trim();
    // `Image | None`, `Optional[Image]`, `list[Image]`, `Image?`
    if let Some((head, _)) = name.split_once('|') {
        return unwrap_annotation(head);
    }
    if let Some(open) = name.find(['[', '<']) {
        let wrapper = name[..open].trim().to_ascii_lowercase();
        let close = if name.ends_with(']') || name.ends_with('>') {
            name.len() - 1
        } else {
            name.len()
        };
        let inside = &name[open + 1..close];
        if matches!(
            wrapper.as_str(),
            "optional" | "list" | "sequence" | "iterable" | "array" | "promise" | "asyncgenerator"
        ) {
            // A tuple is several values, not one wrapped; the caller splits it.
            return unwrap_annotation(inside.split(',').next().unwrap_or(inside));
        }
        return wrapper;
    }
    name.trim_end_matches('?').to_owned()
}

/// The control a defaulted parameter should draw.
///
/// A float between zero and one is a proportion, and a slider is the right
/// control for a proportion — this is what makes SPEC §13.4's threshold a
/// slider. A float outside that range is a quantity (a timeout, a rate) and
/// gets a number field, because a slider needs bounds the code never gave.
pub fn control_for(annotation: Option<&str>, default: &str) -> (Control, Option<f64>, Option<f64>) {
    let hint = annotation.map(|a| unwrap_annotation(a).to_ascii_lowercase());
    let literal = default.trim();

    if matches!(literal, "True" | "true" | "False" | "false") {
        return (Control::Bool, None, None);
    }
    if hint.as_deref() == Some("bool") || hint.as_deref() == Some("boolean") {
        return (Control::Bool, None, None);
    }
    if matches!(hint.as_deref(), Some("path" | "file")) {
        return (Control::Path, None, None);
    }

    if let Ok(number) = literal.parse::<f64>() {
        let fractional = literal.contains('.') || hint.as_deref() == Some("float");
        if fractional && (0.0..=1.0).contains(&number) {
            return (Control::Range, Some(0.0), Some(1.0));
        }
        return (Control::Number, None, None);
    }

    if is_quoted(literal) {
        let text = unquote(literal);
        // A default with a newline in it wants room to hold one.
        if text.contains('\n') || text.len() > 60 {
            return (Control::Multiline, None, None);
        }
        // A default that looks like a path gets the path control, which is
        // what makes `source: str = "analyse.py"` a file picker.
        if text.contains('/') || text.rsplit_once('.').is_some_and(|(_, ext)| ext.len() <= 4) {
            return (Control::Path, None, None);
        }
        return (Control::Text, None, None);
    }
    (Control::Text, None, None)
}

/// Whether a source literal is a quoted string. Public because the engine
/// reads the same defaults this crate parsed, and must read them the same way.
pub fn is_quoted(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() >= 2
        && (bytes[0] == b'"' || bytes[0] == b'\'')
        && bytes[bytes.len() - 1] == bytes[0]
}

/// A quoted source literal without its quotes.
pub fn unquote(text: &str) -> String {
    if is_quoted(text) {
        text[1..text.len() - 1].to_owned()
    } else {
        text.to_owned()
    }
}

/// `max_items` becomes `Max items`; `maxItems` becomes `Max items` too, so the
/// inspector reads the same whichever language wrote the block.
pub fn label_for(name: &str) -> String {
    let mut words = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch == '_' || ch == '-' {
            words.push(' ');
        } else if ch.is_ascii_uppercase() && i > 0 {
            words.push(' ');
            words.push(ch.to_ascii_lowercase());
        } else {
            words.push(ch);
        }
    }
    let trimmed = words.trim();
    let mut out = String::with_capacity(trimmed.len());
    let mut first = true;
    for ch in trimmed.chars() {
        if first {
            out.extend(ch.to_uppercase());
            first = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// A port derived from a parameter.
///
/// Handles are optional. A `tools` or `memory` parameter names a capability
/// the function may use, and a function that is given none should still run —
/// so the port appears, unwired, rather than making the block un-runnable
/// until something is plugged into it (SPEC §4.3, §4.5).
pub(crate) fn port(name: &str, port_type: PortType, side: Side) -> Port {
    Port {
        name: name.to_owned(),
        port_type,
        side,
        optional: port_type.is_closed(),
    }
}

/// The output ports a return annotation describes.
///
/// One value is `result`. A tuple is several, named by position, because the
/// code gave them no names of their own and a positional name is at least
/// honest about where the value came from.
pub(crate) fn outputs(annotation: Option<&str>) -> Vec<Port> {
    let Some(text) = annotation.map(str::trim).filter(|t| !t.is_empty()) else {
        return Vec::new();
    };
    if matches!(text, "None" | "none" | "void" | "null") {
        return Vec::new();
    }
    if let Some(parts) = tuple_members(text) {
        return parts
            .iter()
            .enumerate()
            .map(|(i, member)| {
                port(
                    &format!("result{}", i + 1),
                    port_type(Some(member)),
                    Side::Out,
                )
            })
            .collect();
    }
    vec![port("result", port_type(Some(text)), Side::Out)]
}

/// The members of a tuple annotation, or none when it is not one.
fn tuple_members(text: &str) -> Option<Vec<String>> {
    let lower = text.to_ascii_lowercase();
    let inside = if let Some(rest) = lower.strip_prefix("tuple") {
        rest.trim().strip_prefix('[')?.strip_suffix(']')?
    } else if text.starts_with('[') && text.ends_with(']') {
        &text[1..text.len() - 1]
    } else {
        return None;
    };
    // The lowercased copy is only used to find the wrapper; the members come
    // from the original so `Data` does not become `data`.
    let start = text.len() - inside.len() - 1;
    let members = split_top_level(&text[start..start + inside.len()]);
    (members.len() > 1).then_some(members)
}

/// Split on commas that are not inside brackets, so `dict[str, int]` stays one
/// member rather than becoming two.
pub(crate) fn split_top_level(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for ch in text.chars() {
        match ch {
            '"' | '\'' if quote == Some(ch) => {
                quote = None;
                current.push(ch);
            }
            '"' | '\'' if quote.is_none() => {
                quote = Some(ch);
                current.push(ch);
            }
            _ if quote.is_some() => current.push(ch),
            '[' | '(' | '{' | '<' => {
                depth += 1;
                current.push(ch);
            }
            ']' | ')' | '}' | '>' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                out.push(current.trim().to_owned());
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_owned());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotations_map_onto_the_type_system() {
        assert_eq!(port_type(Some("Image")), PortType::Image);
        assert_eq!(port_type(Some("image")), PortType::Image);
        assert_eq!(port_type(Some("str")), PortType::Text);
        assert_eq!(port_type(Some("dict")), PortType::Data);
        assert_eq!(port_type(Some("Memory")), PortType::Memory);
        // An untyped parameter becomes `any` (SPEC §10.1).
        assert_eq!(port_type(None), PortType::Any);
        // So does a type the grammar has never heard of, which is the honest
        // answer rather than a refusal.
        assert_eq!(port_type(Some("MyDetector")), PortType::Any);
    }

    #[test]
    fn a_wrapper_does_not_change_what_is_inside_it() {
        assert_eq!(port_type(Some("Optional[Image]")), PortType::Image);
        assert_eq!(port_type(Some("Image | None")), PortType::Image);
        assert_eq!(port_type(Some("list[Image]")), PortType::Image);
        assert_eq!(port_type(Some("Promise<Data>")), PortType::Data);
    }

    /// SPEC §13.4: a `float` default becomes a threshold slider. The rule is
    /// the range, not the type.
    #[test]
    fn a_proportion_gets_a_slider_and_a_quantity_gets_a_field() {
        assert_eq!(
            control_for(Some("float"), "0.6"),
            (Control::Range, Some(0.0), Some(1.0))
        );
        assert_eq!(control_for(Some("float"), "30.0").0, Control::Number);
        assert_eq!(control_for(Some("int"), "5").0, Control::Number);
    }

    #[test]
    fn a_bool_default_is_a_switch_in_any_language() {
        assert_eq!(control_for(None, "True").0, Control::Bool);
        assert_eq!(control_for(None, "false").0, Control::Bool);
        assert_eq!(control_for(Some("boolean"), "0").0, Control::Bool);
    }

    #[test]
    fn a_string_default_picks_its_control_from_what_it_looks_like() {
        assert_eq!(control_for(Some("str"), "\"desktop\"").0, Control::Text);
        assert_eq!(control_for(Some("str"), "\"analyse.py\"").0, Control::Path);
        assert_eq!(control_for(Some("str"), "\"/dev/video0\"").0, Control::Path);
        let long = format!("\"{}\"", "x".repeat(80));
        assert_eq!(control_for(Some("str"), &long).0, Control::Multiline);
    }

    #[test]
    fn a_label_reads_the_same_whichever_language_wrote_it() {
        assert_eq!(label_for("threshold"), "Threshold");
        assert_eq!(label_for("max_items"), "Max items");
        assert_eq!(label_for("maxItems"), "Max items");
    }

    #[test]
    fn a_return_annotation_names_the_output() {
        let one = outputs(Some("Data"));
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].name, "result");
        assert_eq!(one[0].port_type, PortType::Data);
        // A function that returns nothing has no output port at all.
        assert!(outputs(Some("None")).is_empty());
        assert!(outputs(None).is_empty());
    }

    #[test]
    fn a_tuple_makes_several_outputs() {
        let many = outputs(Some("tuple[Data, Text]"));
        assert_eq!(many.len(), 2);
        assert_eq!(many[0].name, "result1");
        assert_eq!(many[0].port_type, PortType::Data);
        assert_eq!(many[1].name, "result2");
        assert_eq!(many[1].port_type, PortType::Text);
    }

    /// A generic with a comma inside it is one type, not two.
    #[test]
    fn a_comma_inside_brackets_does_not_split_a_type() {
        assert_eq!(
            split_top_level("a: dict[str, int], b: Image"),
            ["a: dict[str, int]", "b: Image"]
        );
        assert_eq!(
            split_top_level("x = \"a, b\", y = 2"),
            ["x = \"a, b\"", "y = 2"]
        );
    }

    /// A handle is optional: a function given no tools should still run.
    #[test]
    fn a_handle_port_is_optional_and_a_value_port_is_not() {
        assert!(port("memory", PortType::Memory, Side::In).optional);
        assert!(port("tools", PortType::Tools, Side::In).optional);
        assert!(!port("frame", PortType::Image, Side::In).optional);
    }
}
