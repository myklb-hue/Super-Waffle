//! Reading a TypeScript or JavaScript signature (SPEC §10.5).
//!
//! The spelling is `export default block({...}, fn)`, and the runtime strips
//! types, so `.js` and `.ts` are the same block kind. A plain exported function
//! works too: the `block({...})` call carries the icon and the category, and a
//! file that names neither still has a signature to read.

use crate::{
    Control, Generated, Interface, SourceError, control_for, is_quoted, label_for, outputs, port,
    port_type, split_top_level, unquote,
};
use graph_format::{Port, Side};

pub fn parse(source: &str) -> Result<Vec<Interface>, SourceError> {
    let lines: Vec<&str> = source.lines().collect();
    let mut blocks = Vec::new();

    for (i, raw) in lines.iter().enumerate() {
        let line = raw.trim_start();
        let Some(open) = function_open(line) else {
            continue;
        };
        // Only a definition at the left margin, or one exported: a helper
        // nested in a body is not a block.
        if raw.starts_with(char::is_whitespace) && !line.starts_with("export") {
            continue;
        }
        let (signature, _) = gather(&lines, i, open)?;
        blocks.push(read(&lines, i, &signature)?);
    }

    if blocks.is_empty() {
        return Err(SourceError::new(
            1,
            "no function found: a custom block is an exported `function`",
        ));
    }
    Ok(blocks)
}

/// Where the parameter list starts, if this line defines a function.
fn function_open(line: &str) -> Option<usize> {
    let candidates = [
        "export default function ",
        "export async function ",
        "export function ",
        "async function ",
        "function ",
    ];
    for prefix in candidates {
        if let Some(rest) = line.strip_prefix(prefix) {
            return rest.find('(').map(|at| at + prefix.len());
        }
    }
    None
}

fn gather(lines: &[&str], start: usize, _open: usize) -> Result<(String, usize), SourceError> {
    let mut text = String::new();
    let mut depth = 0i32;
    let mut i = start;
    while i < lines.len() {
        for ch in lines[i].chars() {
            match ch {
                '(' | '<' | '[' => depth += 1,
                ')' | '>' | ']' => depth -= 1,
                _ => {}
            }
        }
        text.push_str(lines[i].trim());
        text.push(' ');
        i += 1;
        if depth <= 0 && (text.contains('{') || text.contains("=>")) {
            return Ok((text, i));
        }
    }
    Err(SourceError::new(
        (start + 1) as u32,
        "the signature does not finish: a bracket is left open",
    ))
}

fn read(lines: &[&str], start: usize, signature: &str) -> Result<Interface, SourceError> {
    let line_no = (start + 1) as u32;
    let open = signature
        .find('(')
        .ok_or_else(|| SourceError::new(line_no, "the function has no parameter list"))?;
    let close = matching(signature, open)
        .ok_or_else(|| SourceError::new(line_no, "the parameter list is not closed"))?;

    let name = signature[..open]
        .rsplit(' ')
        .find(|w| !w.is_empty())
        .unwrap_or("block")
        .trim()
        .to_owned();

    let params = &signature[open + 1..close];
    let tail = signature[close + 1..].trim();
    let returns = tail
        .strip_prefix(':')
        .map(|r| r.split('{').next().unwrap_or(r).trim())
        .filter(|r| !r.is_empty() && *r != "void");

    let mut ports: Vec<Port> = Vec::new();
    let mut settings = Vec::new();
    for raw in split_top_level(params) {
        let text = raw.trim();
        if text.is_empty() || text.starts_with("...") {
            continue;
        }
        let (head, default) = match split_default(text) {
            Some((head, default)) => (head, Some(default.trim().to_owned())),
            None => (text, None),
        };
        // `frame?: Image` is an optional parameter, which is still a port.
        let (name_part, annotation) = match head.split_once(':') {
            Some((n, a)) => (n.trim(), Some(a.trim().to_owned())),
            None => (head.trim(), None),
        };
        let param = name_part.trim_end_matches('?').trim();
        if param.is_empty() {
            continue;
        }
        match default {
            None => ports.push(port(param, port_type(annotation.as_deref()), Side::In)),
            Some(default) => {
                let (kind, min, max) = control_for(annotation.as_deref(), &default);
                settings.push(Generated {
                    label: label_for(param),
                    name: param.to_owned(),
                    kind,
                    default,
                    min,
                    max,
                    options: union_options(annotation.as_deref()),
                });
            }
        }
    }
    ports.extend(outputs(returns));

    for setting in &mut settings {
        if !setting.options.is_empty() {
            setting.kind = Control::Select;
        }
    }

    Ok(Interface {
        description: comment_above(lines, start),
        icon: block_option(lines, start, "icon"),
        category: block_option(lines, start, "category"),
        name,
        ports,
        settings,
        line: line_no,
    })
}

fn matching(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    for (i, &b) in bytes.iter().enumerate().skip(open) {
        match b {
            b'(' => depth += 1,
            b')' => {
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

fn split_default(text: &str) -> Option<(&str, &str)> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut quote: Option<u8> = None;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'"' | b'\'' | b'`' if quote == Some(b) => quote = None,
            b'"' | b'\'' | b'`' if quote.is_none() => quote = Some(b),
            _ if quote.is_some() => {}
            b'[' | b'(' | b'{' | b'<' => depth += 1,
            b']' | b')' | b'}' | b'>' => depth -= 1,
            b'=' if depth == 0 => {
                let next = bytes.get(i + 1).copied();
                let prev = i.checked_sub(1).and_then(|p| bytes.get(p)).copied();
                // `=>` is an arrow, not a default.
                if next == Some(b'=')
                    || next == Some(b'>')
                    || matches!(prev, Some(b'=' | b'!' | b'<' | b'>'))
                {
                    continue;
                }
                return Some((&text[..i], &text[i + 1..]));
            }
            _ => {}
        }
    }
    None
}

/// The `//` or `/** */` comment above the function, which is the description.
fn comment_above(lines: &[&str], def: usize) -> Option<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = def;
    while i > 0 {
        i -= 1;
        let line = lines[i].trim();
        if line.is_empty() && out.is_empty() {
            continue;
        }
        let text = line
            .strip_prefix("///")
            .or_else(|| line.strip_prefix("//"))
            .or_else(|| line.strip_prefix("*/"))
            .or_else(|| line.strip_prefix("/**"))
            .or_else(|| line.strip_prefix("/*"))
            .or_else(|| line.strip_prefix('*'));
        match text {
            // A one-line `/** … */` carries its closing marker on the same
            // line, so the opener alone is not enough to strip.
            Some(text) => out.insert(0, text.trim().trim_end_matches("*/").trim().to_owned()),
            None => break,
        }
    }
    let joined = out.join(" ").trim().to_owned();
    (!joined.is_empty()).then_some(joined)
}

/// A key from the `block({ ... })` call anywhere in the file.
fn block_option(lines: &[&str], _def: usize, key: &str) -> Option<String> {
    for line in lines {
        let trimmed = line.trim();
        if !trimmed.contains("block(") {
            continue;
        }
        let open = trimmed.find('{')?;
        let close = trimmed.rfind('}')?;
        for part in split_top_level(&trimmed[open + 1..close]) {
            if let Some((k, v)) = part.split_once(':')
                && k.trim().trim_matches(['"', '\'']) == key
            {
                return Some(unquote(v.trim().trim_end_matches(',')));
            }
        }
    }
    None
}

/// A union of string literals is a choice: `'a' | 'b'`.
fn union_options(annotation: Option<&str>) -> Vec<String> {
    let Some(text) = annotation else {
        return Vec::new();
    };
    let parts: Vec<&str> = text.split('|').map(str::trim).collect();
    if parts.len() < 2 || !parts.iter().all(|p| is_quoted(p)) {
        return Vec::new();
    }
    parts.iter().map(|p| unquote(p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use graph_format::PortType;

    #[test]
    fn the_same_block_in_typescript() {
        let blocks = parse(
            r#"
export default block({ icon: 'shield', category: 'senses' });

/** Is the front door open? */
export function doorCheck(frame: Image, threshold: number = 0.6): Data {
  return { open: detect(frame) > threshold };
}
"#,
        )
        .unwrap();
        let block = &blocks[0];
        assert_eq!(block.name, "doorCheck");
        assert_eq!(
            block.description.as_deref(),
            Some("Is the front door open?")
        );
        assert_eq!(block.icon.as_deref(), Some("shield"));
        assert_eq!(block.category.as_deref(), Some("senses"));
        assert_eq!(block.ports[0].name, "frame");
        assert_eq!(block.ports[0].port_type, PortType::Image);
        assert_eq!(block.ports[1].name, "result");
        assert_eq!(block.ports[1].port_type, PortType::Data);
        assert_eq!(block.settings[0].kind, Control::Range);
        assert_eq!(block.settings[0].label, "Threshold");
    }

    /// The runtime strips types, so a plain `.js` file with no annotations is
    /// the same block kind — every port is `any`, which is what it is.
    #[test]
    fn plain_javascript_is_the_same_block_kind() {
        let blocks = parse("export function classify(text, threshold = 0.5) {\n}\n").unwrap();
        assert_eq!(blocks[0].name, "classify");
        assert_eq!(blocks[0].ports[0].name, "text");
        assert_eq!(blocks[0].ports[0].port_type, PortType::Any);
        assert_eq!(blocks[0].settings[0].name, "threshold");
    }

    #[test]
    fn a_union_of_strings_is_a_choice() {
        let blocks = parse(
            "export function n(t: Text, target: 'desktop' | 'slack' = 'desktop'): void {\n}\n",
        )
        .unwrap();
        assert_eq!(blocks[0].settings[0].kind, Control::Select);
        assert_eq!(blocks[0].settings[0].options, ["desktop", "slack"]);
        // `void` means no output port.
        assert_eq!(blocks[0].ports.len(), 1);
    }

    /// A default holding an arrow function must not be split at its `=>`.
    #[test]
    fn an_arrow_in_a_default_is_not_a_default_marker() {
        let blocks = parse("export function f(a: Text, fn = (x) => x + 1): Data {\n}\n").unwrap();
        assert_eq!(blocks[0].settings.len(), 1);
        assert_eq!(blocks[0].settings[0].name, "fn");
    }

    #[test]
    fn an_optional_parameter_is_still_a_port() {
        let blocks = parse("export function f(frame?: Image): Data {\n}\n").unwrap();
        assert_eq!(blocks[0].ports[0].name, "frame");
        assert_eq!(blocks[0].ports[0].port_type, PortType::Image);
    }

    #[test]
    fn a_file_with_no_function_says_so() {
        let error = parse("const x = 1;\n").unwrap_err();
        assert!(error.message.contains("function"), "{error}");
    }
}
