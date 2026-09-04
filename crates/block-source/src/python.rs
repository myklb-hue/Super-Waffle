//! Reading a Python signature (SPEC §10.1, §10.5).
//!
//! Finds `def name(params) -> Return:`, the `@block(...)` decorator above it
//! and the docstring below it. Nothing else: the body is not read, not
//! validated, and not required to be finished.

use crate::{
    Control, Generated, Interface, SourceError, control_for, is_quoted, label_for, outputs, port,
    port_type, split_top_level, unquote,
};
use graph_format::{Port, Side};

pub fn parse(source: &str) -> Result<Vec<Interface>, SourceError> {
    let lines: Vec<&str> = source.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0usize;

    while i < lines.len() {
        let line = lines[i].trim_start();
        if !line.starts_with("def ") && !line.starts_with("async def ") {
            i += 1;
            continue;
        }
        // A nested function is part of a body, not a block. Only a definition
        // at the left margin is one.
        if lines[i].starts_with(char::is_whitespace) {
            i += 1;
            continue;
        }

        let start = i;
        let (signature, after) = gather_signature(&lines, i)?;
        let interface = read(&lines, start, &signature, after)?;
        blocks.push(interface);
        i = after;
    }

    if blocks.is_empty() {
        return Err(SourceError::new(
            1,
            "no function found: a custom block is a `def` at the left margin",
        ));
    }
    Ok(blocks)
}

/// Collect a signature that may run over several lines, and say where the body
/// starts.
fn gather_signature(lines: &[&str], start: usize) -> Result<(String, usize), SourceError> {
    let mut text = String::new();
    let mut depth = 0i32;
    let mut i = start;
    while i < lines.len() {
        let line = lines[i];
        for ch in line.chars() {
            match ch {
                '(' | '[' => depth += 1,
                ')' | ']' => depth -= 1,
                '#' if depth == 0 => break,
                _ => {}
            }
        }
        text.push_str(line.trim());
        text.push(' ');
        i += 1;
        if depth <= 0 && text.contains(':') {
            return Ok((text, i));
        }
        if depth < 0 {
            break;
        }
    }
    Err(SourceError::new(
        (start + 1) as u32,
        "the signature does not finish: a bracket is left open",
    ))
}

fn read(
    lines: &[&str],
    start: usize,
    signature: &str,
    body: usize,
) -> Result<Interface, SourceError> {
    let line_no = (start + 1) as u32;

    let after_def = signature
        .trim_start()
        .strip_prefix("async ")
        .unwrap_or(signature.trim_start())
        .strip_prefix("def ")
        .ok_or_else(|| SourceError::new(line_no, "expected `def`"))?;

    let open = after_def
        .find('(')
        .ok_or_else(|| SourceError::new(line_no, "the function has no parameter list"))?;
    let name = after_def[..open].trim().to_owned();
    if name.is_empty() {
        return Err(SourceError::new(line_no, "the function has no name"));
    }

    let close = matching(after_def, open)
        .ok_or_else(|| SourceError::new(line_no, "the parameter list is not closed"))?;
    let params = &after_def[open + 1..close];

    // Everything after the parameter list, up to the colon that opens the
    // body, is the return annotation.
    let tail = after_def[close + 1..].trim();
    let returns = tail
        .strip_prefix("->")
        .map(|r| r.trim_end().trim_end_matches(':').trim())
        .filter(|r| !r.is_empty());

    let mut ports: Vec<Port> = Vec::new();
    let mut settings = Vec::new();
    for raw in split_top_level(params) {
        let Some(param) = Parameter::read(&raw) else {
            continue;
        };
        match param.default {
            // A parameter without a default is an input port, typed by its
            // annotation (SPEC §10.1).
            None => ports.push(port(
                &param.name,
                port_type(param.annotation.as_deref()),
                Side::In,
            )),
            // One with a default is a setting.
            Some(default) => {
                let (kind, min, max) = control_for(param.annotation.as_deref(), &default);
                settings.push(Generated {
                    label: label_for(&param.name),
                    name: param.name,
                    kind,
                    default: default.clone(),
                    min,
                    max,
                    options: literal_options(param.annotation.as_deref()),
                });
            }
        }
    }
    ports.extend(outputs(returns));

    // A `Literal[...]` annotation turns its setting into a choice.
    for setting in &mut settings {
        if !setting.options.is_empty() {
            setting.kind = Control::Select;
        }
    }

    Ok(Interface {
        description: docstring(lines, body),
        icon: decorator_value(lines, start, "icon"),
        category: decorator_value(lines, start, "category"),
        name,
        ports,
        settings,
        line: line_no,
    })
}

struct Parameter {
    name: String,
    annotation: Option<String>,
    default: Option<String>,
}

impl Parameter {
    fn read(raw: &str) -> Option<Self> {
        let text = raw.trim();
        // `self`, `*args`, `**kwargs` and the bare `*` and `/` markers describe
        // how the function is called, not what it takes.
        if text.is_empty() || text == "self" || text == "*" || text == "/" {
            return None;
        }
        if text.starts_with('*') {
            return None;
        }

        let (head, default) = match split_default(text) {
            Some((head, default)) => (head, Some(default.trim().to_owned())),
            None => (text, None),
        };
        let (name, annotation) = match head.split_once(':') {
            Some((name, annotation)) => (name.trim(), Some(annotation.trim().to_owned())),
            None => (head.trim(), None),
        };
        (!name.is_empty()).then(|| Self {
            name: name.to_owned(),
            annotation,
            default,
        })
    }
}

/// Split on the `=` that introduces a default, ignoring the ones inside a
/// nested call or a string.
fn split_default(text: &str) -> Option<(&str, &str)> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut quote: Option<u8> = None;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'"' | b'\'' if quote == Some(b) => quote = None,
            b'"' | b'\'' if quote.is_none() => quote = Some(b),
            _ if quote.is_some() => {}
            b'[' | b'(' | b'{' => depth += 1,
            b']' | b')' | b'}' => depth -= 1,
            // `==`, `<=` and `>=` are comparisons, and `=` inside brackets is
            // a keyword argument in someone else's call.
            b'=' if depth == 0 => {
                let next = bytes.get(i + 1).copied();
                let prev = i.checked_sub(1).and_then(|p| bytes.get(p)).copied();
                if next == Some(b'=') || matches!(prev, Some(b'=' | b'!' | b'<' | b'>')) {
                    continue;
                }
                return Some((&text[..i], &text[i + 1..]));
            }
            _ => {}
        }
    }
    None
}

/// The index of the bracket closing the one at `open`.
fn matching(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    for (i, &b) in bytes.iter().enumerate().skip(open) {
        match b {
            b'(' | b'[' => depth += 1,
            b')' | b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// The first string in the body, which is the description (SPEC §10.1).
fn docstring(lines: &[&str], body: usize) -> Option<String> {
    let first = lines.get(body)?.trim();
    for fence in ["\"\"\"", "'''"] {
        if let Some(rest) = first.strip_prefix(fence) {
            // All on one line.
            if let Some(text) = rest.strip_suffix(fence) {
                return Some(text.trim().to_owned());
            }
            let mut out = vec![rest.trim().to_owned()];
            for line in lines.iter().skip(body + 1) {
                let trimmed = line.trim();
                if let Some(last) = trimmed.strip_suffix(fence) {
                    if !last.trim().is_empty() {
                        out.push(last.trim().to_owned());
                    }
                    break;
                }
                out.push(trimmed.to_owned());
            }
            let joined = out.join(" ").trim().to_owned();
            return (!joined.is_empty()).then_some(joined);
        }
    }
    if is_quoted(first) {
        return Some(unquote(first));
    }
    None
}

/// A keyword from the `@block(...)` decorator above the function.
fn decorator_value(lines: &[&str], def: usize, key: &str) -> Option<String> {
    // Decorators sit directly above, possibly several of them.
    let mut i = def;
    while i > 0 {
        i -= 1;
        let line = lines[i].trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if !line.starts_with('@') {
            return None;
        }
        if let Some(open) = line.find('(') {
            for part in split_top_level(&line[open + 1..line.rfind(')').unwrap_or(line.len())]) {
                if let Some((k, v)) = part.split_once('=')
                    && k.trim() == key
                {
                    return Some(unquote(v.trim()));
                }
            }
        }
    }
    None
}

/// The choices in a `Literal["a", "b"]` annotation.
fn literal_options(annotation: Option<&str>) -> Vec<String> {
    let Some(text) = annotation else {
        return Vec::new();
    };
    let text = text.trim();
    let Some(rest) = text
        .strip_prefix("Literal[")
        .or_else(|| text.strip_prefix("literal["))
    else {
        return Vec::new();
    };
    let Some(inside) = rest.strip_suffix(']') else {
        return Vec::new();
    };
    split_top_level(inside)
        .iter()
        .filter(|p| is_quoted(p))
        .map(|p| unquote(p))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use graph_format::PortType;

    /// SPEC §13.4's own example, which is also the door-watch fixture.
    const DOOR_CHECK: &str = r#"
def door_check(frame: Image, threshold: float = 0.6) -> Data:
    """Is the front door open?"""
    score = detect(frame)
    return {"open": score > threshold}
"#;

    #[test]
    fn the_signature_is_the_block() {
        let blocks = parse(DOOR_CHECK).unwrap();
        assert_eq!(blocks.len(), 1);
        let block = &blocks[0];

        assert_eq!(block.name, "door_check");
        assert_eq!(
            block.description.as_deref(),
            Some("Is the front door open?")
        );

        // One input port from the undefaulted parameter, one output from the
        // return annotation.
        assert_eq!(block.ports.len(), 2);
        assert_eq!(block.ports[0].name, "frame");
        assert_eq!(block.ports[0].port_type, PortType::Image);
        assert_eq!(block.ports[0].side, Side::In);
        assert_eq!(block.ports[1].name, "result");
        assert_eq!(block.ports[1].port_type, PortType::Data);
        assert_eq!(block.ports[1].side, Side::Out);

        // And a threshold slider from the defaulted one.
        assert_eq!(block.settings.len(), 1);
        assert_eq!(block.settings[0].name, "threshold");
        assert_eq!(block.settings[0].label, "Threshold");
        assert_eq!(block.settings[0].kind, Control::Range);
        assert_eq!(block.settings[0].default, "0.6");
        assert_eq!(block.settings[0].max, Some(1.0));
    }

    /// The decorator decides the name, the icon and the shelf (SPEC §10.1).
    #[test]
    fn the_decorator_says_which_shelf_it_lands_on() {
        let blocks = parse(
            r#"
@block(icon="shield", category="senses")
def door_check(frame: Image) -> Data:
    pass
"#,
        )
        .unwrap();
        assert_eq!(blocks[0].icon.as_deref(), Some("shield"));
        assert_eq!(blocks[0].category.as_deref(), Some("senses"));
    }

    /// A file with several decorated functions makes several blocks
    /// (SPEC §10.5).
    #[test]
    fn several_functions_make_several_blocks() {
        let blocks = parse(
            r#"
def first(a: Text) -> Data:
    """One."""
    pass

def second(b: Image) -> Text:
    """Two."""
    pass
"#,
        )
        .unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].name, "first");
        assert_eq!(blocks[1].name, "second");
        // Each knows where it starts, so an error points at the right one.
        assert_eq!(blocks[0].line, 2);
        assert_eq!(blocks[1].line, 6);
    }

    /// A function inside another function is part of a body, not a block.
    #[test]
    fn a_nested_function_is_not_a_block() {
        let blocks = parse(
            r#"
def outer(a: Text) -> Data:
    def helper(x):
        return x
    return helper(a)
"#,
        )
        .unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].name, "outer");
    }

    /// A signature over several lines is one signature.
    #[test]
    fn a_wrapped_signature_reads_as_one() {
        let blocks = parse(
            r#"
def analyse(
    frame: Image,
    memory: Memory,
    threshold: float = 0.8,
) -> tuple[Data, Text]:
    pass
"#,
        )
        .unwrap();
        let block = &blocks[0];
        let names: Vec<&str> = block.ports.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["frame", "memory", "result1", "result2"]);
        // A Memory parameter gives the block a memory handle (SPEC §10.1),
        // and a handle is optional.
        assert_eq!(block.ports[1].port_type, PortType::Memory);
        assert!(block.ports[1].optional);
        assert_eq!(block.settings.len(), 1);
    }

    /// `Literal` is a choice, and the inspector should draw it as one.
    #[test]
    fn a_literal_annotation_becomes_a_choice() {
        let blocks = parse(
            r#"
def notify(text: Text, target: Literal["desktop", "slack", "email"] = "desktop") -> None:
    pass
"#,
        )
        .unwrap();
        let setting = &blocks[0].settings[0];
        assert_eq!(setting.kind, Control::Select);
        assert_eq!(setting.options, ["desktop", "slack", "email"]);
        // Returning None means no output port at all.
        assert_eq!(blocks[0].ports.len(), 1);
    }

    /// The default is kept exactly as written, because editing the setting
    /// rewrites this text in the code (SPEC §10.3).
    #[test]
    fn a_default_keeps_the_spelling_the_code_used() {
        let blocks = parse("def f(x: float = 0.60) -> Data:\n    pass\n").unwrap();
        assert_eq!(blocks[0].settings[0].default, "0.60");
    }

    /// A default containing a comma or an `=` is one default.
    #[test]
    fn a_tricky_default_does_not_split_the_parameter_list() {
        let blocks = parse(
            r#"def f(a: Text, size: Data = {"w": 1, "h": 2}, tag: str = "a=b") -> Data:
    pass
"#,
        )
        .unwrap();
        assert_eq!(
            blocks[0]
                .ports
                .iter()
                .filter(|p| p.side == Side::In)
                .count(),
            1
        );
        assert_eq!(blocks[0].settings.len(), 2);
        assert_eq!(blocks[0].settings[1].default, "\"a=b\"");
    }

    /// Nothing to parse says so, rather than producing a block with no ports
    /// that looks like it worked.
    #[test]
    fn a_file_with_no_function_is_an_error_that_says_what_is_missing() {
        let error = parse("x = 1\n").unwrap_err();
        assert!(error.message.contains("def"), "{error}");
        assert_eq!(error.line, 1);
    }

    /// An unfinished signature is reported at its own line, which is what the
    /// block shows while someone is still typing (SPEC §10.4).
    #[test]
    fn an_unclosed_signature_points_at_its_line() {
        let error = parse("\n\ndef f(a: Text,\n").unwrap_err();
        assert_eq!(error.line, 3);
        assert!(error.message.contains("bracket"), "{error}");
    }

    /// An untyped parameter is still a port, typed `any` (SPEC §10.1).
    #[test]
    fn an_untyped_parameter_is_an_any_port() {
        let blocks = parse("def f(value) -> Data:\n    pass\n").unwrap();
        assert_eq!(blocks[0].ports[0].name, "value");
        assert_eq!(blocks[0].ports[0].port_type, PortType::Any);
    }

    /// `self`, `*args` and `**kwargs` describe how the function is called, not
    /// what it takes.
    #[test]
    fn calling_conventions_are_not_ports() {
        let blocks = parse("def f(self, a: Text, *args, **kwargs) -> Data:\n    pass\n").unwrap();
        let ins: Vec<&str> = blocks[0]
            .ports
            .iter()
            .filter(|p| p.side == Side::In)
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(ins, ["a"]);
    }

    #[test]
    fn a_multi_line_docstring_becomes_one_description() {
        let blocks = parse(
            "def f(a: Text) -> Data:\n    \"\"\"First line.\n    Second line.\n    \"\"\"\n    pass\n",
        )
        .unwrap();
        assert_eq!(
            blocks[0].description.as_deref(),
            Some("First line. Second line.")
        );
    }

    #[test]
    fn an_async_function_is_a_block_like_any_other() {
        let blocks = parse("async def fetch(url: Text) -> Data:\n    pass\n").unwrap();
        assert_eq!(blocks[0].name, "fetch");
        assert_eq!(blocks[0].ports.len(), 2);
    }
}
