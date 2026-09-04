//! Reading a shell block (SPEC §10.5).
//!
//! A shell script has no signature to read, so it declares its interface in
//! comments: `# @block`, `# @in name: type`, `# @out name: type`. That is the
//! one language where the rule in §10.1 cannot hold literally — there is no
//! parameter list — and the comments are how it holds in spirit: the interface
//! is still in the file, still next to the code, and still the only copy.

use crate::{Generated, Interface, SourceError, control_for, label_for, port, port_type, unquote};
use graph_format::Side;

pub fn parse(source: &str) -> Result<Vec<Interface>, SourceError> {
    let mut blocks: Vec<Interface> = Vec::new();

    for (i, raw) in source.lines().enumerate() {
        let line = raw.trim();
        let Some(rest) = line.strip_prefix('#').map(str::trim) else {
            continue;
        };
        let line_no = (i + 1) as u32;

        if let Some(after) = rest.strip_prefix("@block") {
            blocks.push(Interface {
                name: attribute(after, "name").unwrap_or_else(|| {
                    after
                        .split_whitespace()
                        .next()
                        .unwrap_or("block")
                        .to_owned()
                }),
                description: None,
                icon: attribute(after, "icon"),
                category: attribute(after, "category"),
                ports: Vec::new(),
                settings: Vec::new(),
                line: line_no,
            });
            continue;
        }

        let Some(block) = blocks.last_mut() else {
            // A declaration before any `@block` has nothing to attach to, and
            // saying so beats silently ignoring the line someone just wrote.
            if rest.starts_with("@in") || rest.starts_with("@out") || rest.starts_with("@set") {
                return Err(SourceError::new(
                    line_no,
                    "this declaration comes before any `# @block`",
                ));
            }
            continue;
        };

        if let Some(decl) = rest.strip_prefix("@in ") {
            let (name, annotation) = split_declaration(decl);
            block
                .ports
                .push(port(&name, port_type(annotation.as_deref()), Side::In));
        } else if let Some(decl) = rest.strip_prefix("@out ") {
            let (name, annotation) = split_declaration(decl);
            block
                .ports
                .push(port(&name, port_type(annotation.as_deref()), Side::Out));
        } else if let Some(decl) = rest.strip_prefix("@set ") {
            let (head, default) = match decl.split_once('=') {
                Some((head, default)) => (head, default.trim().to_owned()),
                None => (decl, String::new()),
            };
            let (name, annotation) = split_declaration(head);
            let (kind, min, max) = control_for(annotation.as_deref(), &default);
            block.settings.push(Generated {
                label: label_for(&name),
                name,
                kind,
                default,
                min,
                max,
                options: Vec::new(),
            });
        } else if let Some(text) = rest.strip_prefix("@doc ") {
            block.description = Some(unquote(text.trim()));
        }
    }

    if blocks.is_empty() {
        return Err(SourceError::new(
            1,
            "no block found: a shell block starts with a `# @block` comment",
        ));
    }
    Ok(blocks)
}

/// `name: type`, or just a name.
fn split_declaration(text: &str) -> (String, Option<String>) {
    match text.split_once(':') {
        Some((name, annotation)) => (name.trim().to_owned(), Some(annotation.trim().to_owned())),
        None => (text.trim().to_owned(), None),
    }
}

/// `key=value` on the `@block` line.
fn attribute(text: &str, key: &str) -> Option<String> {
    for part in text.split_whitespace() {
        if let Some((k, v)) = part.split_once('=')
            && k == key
        {
            return Some(unquote(v));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Control;
    use graph_format::PortType;

    #[test]
    fn a_shell_block_declares_its_interface_in_comments() {
        let blocks = parse(
            r#"#!/bin/sh
# @block backup icon=folder category=runtimes
# @doc Copies the day's files somewhere safe.
# @in source: File
# @out result: Data
# @set keep: int = 7
tar czf "$1.tgz" "$1"
"#,
        )
        .unwrap();
        let block = &blocks[0];
        assert_eq!(block.name, "backup");
        assert_eq!(block.icon.as_deref(), Some("folder"));
        assert_eq!(block.category.as_deref(), Some("runtimes"));
        assert_eq!(
            block.description.as_deref(),
            Some("Copies the day's files somewhere safe.")
        );
        assert_eq!(block.ports.len(), 2);
        assert_eq!(block.ports[0].port_type, PortType::File);
        assert_eq!(block.ports[0].side, Side::In);
        assert_eq!(block.ports[1].side, Side::Out);
        assert_eq!(block.settings[0].name, "keep");
        assert_eq!(block.settings[0].default, "7");
        assert_eq!(block.settings[0].kind, Control::Number);
    }

    #[test]
    fn several_blocks_in_one_script() {
        let blocks = parse("# @block one\n# @in a: Text\n# @block two\n# @in b: Image\n").unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].ports.len(), 1);
        assert_eq!(blocks[1].ports[0].port_type, PortType::Image);
    }

    /// A declaration with nothing to attach to is reported rather than
    /// dropped: silence would look exactly like a typo that worked.
    #[test]
    fn a_port_before_any_block_is_an_error_at_its_line() {
        let error = parse("#!/bin/sh\n# @in a: Text\n# @block late\n").unwrap_err();
        assert_eq!(error.line, 2);
        assert!(error.message.contains("@block"), "{error}");
    }

    #[test]
    fn a_script_with_no_declaration_says_what_is_missing() {
        let error = parse("echo hello\n").unwrap_err();
        assert!(error.message.contains("@block"), "{error}");
    }
}
