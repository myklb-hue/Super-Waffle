//! The canonical emitter.
//!
//! Reading a `.loom` file and writing it back must produce the same bytes, and
//! a graph edited by hand and one edited on the canvas must produce the same
//! file (SPEC §15.3). That is why this is written by hand rather than handed to
//! a general YAML serialiser: the key order, the quoting, the block scalars and
//! the number formatting are all decisions the format makes, not the library.
//!
//! The rules:
//!   * Keys appear in the order this file writes them, never alphabetically.
//!   * Setting and env keys come out sorted, because they are held in a
//!     `BTreeMap`: their order carries no meaning, and sorting is what makes
//!     two hands produce one file.
//!   * Positions are rounded to the grid.
//!   * Empty optional collections are omitted, not written as `[]` or `{}`.
//!   * Inline code is a literal block scalar, so a diff reads as code.

use crate::model::*;
use std::fmt::Write as _;

/// Write a graph as its canonical `.loom` text.
pub fn to_string(graph: &Graph) -> String {
    let mut out = String::with_capacity(4096);
    let g = graph;

    line(&mut out, 0, &format!("version: {}", g.version));
    line(&mut out, 0, &format!("id: {}", scalar(&g.id)));
    line(&mut out, 0, &format!("name: {}", scalar(&g.name)));
    if let Some(d) = &g.description {
        write_text_field(&mut out, 0, "description", d);
    }
    line(&mut out, 0, &format!("runMode: {}", g.run_mode.as_str()));
    line(&mut out, 0, &format!("localOnly: {}", g.local_only));

    line(&mut out, 0, "execution:");
    line(
        &mut out,
        1,
        &format!("runtime: {}", scalar(&g.execution.runtime)),
    );
    line(
        &mut out,
        1,
        &format!("concurrency: {}", g.execution.concurrency),
    );
    line(
        &mut out,
        1,
        &format!("timeoutSec: {}", g.execution.timeout_sec),
    );

    line(&mut out, 0, "defaults:");
    line(
        &mut out,
        1,
        &format!("provider: {}", scalar(&g.defaults.provider)),
    );
    line(
        &mut out,
        1,
        &format!("model: {}", scalar(&g.defaults.model)),
    );

    line(&mut out, 0, "overlap:");
    line(
        &mut out,
        1,
        &format!("policy: {}", g.overlap.policy.as_str()),
    );
    line(&mut out, 1, &format!("maxQueue: {}", g.overlap.max_queue));
    line(
        &mut out,
        1,
        &format!("coalesceMs: {}", g.overlap.coalesce_ms),
    );
    line(
        &mut out,
        1,
        &format!("loopParallel: {}", g.overlap.loop_parallel),
    );

    line(&mut out, 0, "between:");
    line(&mut out, 1, &format!("keepState: {}", g.between.keep_state));
    line(
        &mut out,
        1,
        &format!("restartOnCrash: {}", g.between.restart_on_crash),
    );

    if !g.env.is_empty() {
        line(&mut out, 0, "env:");
        for (k, v) in &g.env {
            line(&mut out, 1, &format!("{}: {}", scalar(k), scalar(v)));
        }
    }

    line(&mut out, 0, "blocks:");
    for b in &g.blocks {
        write_block(&mut out, b);
    }

    if !g.frames.is_empty() {
        line(&mut out, 0, "frames:");
        for f in &g.frames {
            write_frame(&mut out, f);
        }
    }

    line(&mut out, 0, "wires:");
    for w in &g.wires {
        line(&mut out, 1, &format!("- id: {}", scalar(&w.id)));
        line(&mut out, 2, &format!("from: {}", scalar(&w.from.to_ref())));
        line(&mut out, 2, &format!("to: {}", scalar(&w.to.to_ref())));
    }

    line(&mut out, 0, "ui:");
    line(
        &mut out,
        1,
        &format!(
            "viewport: [{}, {}, {}]",
            num(g.ui.viewport.x),
            num(g.ui.viewport.y),
            num(g.ui.viewport.zoom)
        ),
    );

    out
}

fn write_block(out: &mut String, b: &Block) {
    let p = b.position.snapped();
    line(out, 1, &format!("- id: {}", scalar(&b.id)));
    line(out, 2, &format!("kind: {}", scalar(&b.kind)));
    if let Some(t) = &b.title {
        line(out, 2, &format!("title: {}", scalar(t)));
    }
    line(out, 2, &format!("position: [{}, {}]", num(p.x), num(p.y)));
    if let Some(s) = &b.size {
        match s.h {
            Some(h) => line(out, 2, &format!("size: [{}, {}]", num(s.w), num(h))),
            None => line(out, 2, &format!("size: [{}]", num(s.w))),
        }
    }
    line(out, 2, &format!("view: {}", b.view.as_str()));
    if b.disabled {
        line(out, 2, "disabled: true");
    }
    if b.breakpoint {
        line(out, 2, "breakpoint: true");
    }
    if let Some(f) = &b.frame {
        line(out, 2, &format!("frame: {}", scalar(f)));
    }
    if !b.settings.is_empty() {
        line(out, 2, "settings:");
        for (k, v) in &b.settings {
            write_value(out, 3, k, v);
        }
    }
    if !b.ports.is_empty() {
        line(out, 2, "ports:");
        for port in &b.ports {
            let optional = if port.optional {
                ", optional: true"
            } else {
                ""
            };
            line(
                out,
                3,
                &format!(
                    "- {{ name: {}, type: {}, side: {}{} }}",
                    scalar(&port.name),
                    port.port_type,
                    match port.side {
                        crate::types::Side::In => "in",
                        crate::types::Side::Out => "out",
                    },
                    optional
                ),
            );
        }
    }
    if let Some(s) = &b.source {
        line(out, 2, "source:");
        line(out, 3, &format!("mode: {}", s.mode.as_str()));
        line(out, 3, &format!("language: {}", s.language.as_str()));
        if let Some(path) = &s.path {
            line(out, 3, &format!("path: {}", scalar(path)));
        }
        if let Some(code) = &s.code {
            write_block_scalar(out, 3, "code", code);
        }
    }
}

fn write_frame(out: &mut String, f: &Frame) {
    let p = f.position.snapped();
    line(out, 1, &format!("- id: {}", scalar(&f.id)));
    line(out, 2, &format!("kind: {}", f.kind.as_str()));
    line(out, 2, &format!("position: [{}, {}]", num(p.x), num(p.y)));
    line(
        out,
        2,
        &format!(
            "size: [{}, {}]",
            num(f.size.w),
            num(f.size.h.unwrap_or(0.0))
        ),
    );
    line(out, 2, &format!("over: {}", scalar(&f.over.to_ref())));
    line(out, 2, &format!("as: {}", scalar(&f.as_name)));
    line(out, 2, &format!("parallel: {}", f.parallel));
    line(out, 2, &format!("max: {}", f.max));
    if let Some(s) = &f.stop_when {
        line(out, 2, &format!("stopWhen: {}", scalar(&s.to_ref())));
    }
    line(out, 2, &format!("continueOnError: {}", f.continue_on_error));
}

fn write_value(out: &mut String, depth: usize, key: &str, v: &Setting) {
    match v {
        Setting::Map(m) if !m.is_empty() => {
            line(out, depth, &format!("{}:", scalar(key)));
            for (k, v) in m {
                write_value(out, depth + 1, k, v);
            }
        }
        Setting::List(items) if !items.is_empty() => {
            line(out, depth, &format!("{}:", scalar(key)));
            for item in items {
                line(out, depth + 1, &format!("- {}", inline_value(item)));
            }
        }
        // A string with a newline reads as code or prose, so give it a block
        // scalar rather than an escaped one-liner.
        Setting::String(s) if s.contains('\n') => write_block_scalar(out, depth, key, s),
        _ => line(out, depth, &format!("{}: {}", scalar(key), inline_value(v))),
    }
}

fn inline_value(v: &Setting) -> String {
    match v {
        Setting::Null => "null".into(),
        Setting::Bool(b) => b.to_string(),
        Setting::Int(i) => i.to_string(),
        Setting::Float(f) => num(*f),
        Setting::String(s) => scalar(s),
        Setting::List(items) => {
            let inner: Vec<_> = items.iter().map(inline_value).collect();
            format!("[{}]", inner.join(", "))
        }
        Setting::Map(m) => {
            let inner: Vec<_> = m
                .iter()
                .map(|(k, v)| format!("{}: {}", scalar(k), inline_value(v)))
                .collect();
            format!("{{ {} }}", inner.join(", "))
        }
    }
}

/// A multi-line string as a literal block scalar. `|-` strips the trailing
/// newline, `|` keeps exactly one; we always normalise to one so that code
/// with and without a final newline cannot produce two different files.
fn write_block_scalar(out: &mut String, depth: usize, key: &str, text: &str) {
    line(out, depth, &format!("{}: |", scalar(key)));
    let body = text.strip_suffix('\n').unwrap_or(text);
    for l in body.split('\n') {
        if l.is_empty() {
            out.push('\n');
        } else {
            line(out, depth + 1, l);
        }
    }
}

/// Prose: one line if it is short and plain, a block scalar if it is not.
fn write_text_field(out: &mut String, depth: usize, key: &str, text: &str) {
    if text.contains('\n') {
        write_block_scalar(out, depth, key, text);
    } else {
        line(out, depth, &format!("{}: {}", scalar(key), scalar(text)));
    }
}

fn line(out: &mut String, depth: usize, text: &str) {
    for _ in 0..depth {
        out.push_str("  ");
    }
    out.push_str(text);
    out.push('\n');
}

/// Numbers are written as integers when they are integral, so a position of
/// 66.0 is `66` and not `66.0`.
fn num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        let mut s = format!("{v}");
        if !s.contains('.') && !s.contains('e') {
            let _ = write!(s, ".0");
        }
        s
    }
}

/// Quote a scalar only when leaving it bare would change its meaning.
fn scalar(s: &str) -> String {
    if needs_quotes(s) {
        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        format!("\"{escaped}\"")
    } else {
        s.to_owned()
    }
}

fn needs_quotes(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    // Anything YAML would read as another type, or that could start a
    // structure, has to be quoted.
    const RESERVED: [&str; 22] = [
        "true", "false", "null", "yes", "no", "on", "off", "y", "n", "~", "True", "False", "Null",
        "Yes", "No", "On", "Off", "TRUE", "FALSE", "NULL", "Y", "N",
    ];
    if RESERVED.contains(&s) {
        return true;
    }
    if s.parse::<f64>().is_ok() || s.parse::<i64>().is_ok() {
        return true;
    }
    let first = s.chars().next().unwrap();
    if "-?:,[]{}#&*!|>'\"%@` \t".contains(first) {
        return true;
    }
    if s.ends_with(' ') || s.ends_with('\t') {
        return true;
    }
    // A colon followed by a space starts a mapping; a ` #` starts a comment.
    s.contains(": ") || s.contains(" #") || s.contains('\n') || s.ends_with(':')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integral_numbers_lose_their_point() {
        assert_eq!(num(66.0), "66");
        assert_eq!(num(-22.0), "-22");
        assert_eq!(num(1.5), "1.5");
        assert_eq!(num(0.6), "0.6");
    }

    #[test]
    fn scalars_are_quoted_only_when_they_must_be() {
        assert_eq!(scalar("llm"), "llm");
        assert_eq!(scalar("llama3.2:3b"), "llama3.2:3b");
        assert_eq!(scalar("Customer triage"), "Customer triage");
        assert_eq!(scalar("true"), "\"true\"");
        assert_eq!(scalar("42"), "\"42\"");
        assert_eq!(scalar(""), "\"\"");
        assert_eq!(scalar("- leading dash"), "\"- leading dash\"");
        assert_eq!(scalar("key: value"), "\"key: value\"");
    }

    #[test]
    fn block_scalars_normalise_the_trailing_newline() {
        let mut a = String::new();
        write_block_scalar(&mut a, 0, "code", "one\ntwo\n");
        let mut b = String::new();
        write_block_scalar(&mut b, 0, "code", "one\ntwo");
        assert_eq!(a, b);
        assert_eq!(a, "code: |\n  one\n  two\n");
    }
}
