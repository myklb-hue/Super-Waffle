//! Reading a `.loom` file.
//!
//! Parsing is strict and hand-written rather than derived: a file that is
//! nearly right should say which key is wrong and where, not quietly fill in a
//! default. Every error names the path it failed at (`blocks[2].position`),
//! because that is what makes a bad file fixable by hand.

use crate::model::*;
use crate::types::*;
use saphyr::{LoadableYamlNode, Yaml};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("not valid YAML: {0}")]
    Yaml(String),
    #[error("a .loom file must be one mapping at the top level")]
    NotAMapping,
    #[error("unsupported format version {found}; this build reads version {expected}")]
    Version { found: u32, expected: u32 },
    #[error("{path}: missing")]
    Missing { path: String },
    #[error("{path}: expected {expected}")]
    Type {
        path: String,
        expected: &'static str,
    },
    #[error("{path}: {message}")]
    Invalid { path: String, message: String },
}

type R<T> = Result<T, ReadError>;

/// A node plus the path that reached it, so an error can name its location.
struct Node<'a> {
    yaml: &'a Yaml<'a>,
    path: String,
}

/// Parse the text of a graph.
pub fn from_str(text: &str) -> R<Graph> {
    let docs = Yaml::load_from_str(text).map_err(|e| ReadError::Yaml(e.to_string()))?;
    let doc = docs.into_iter().next().ok_or(ReadError::NotAMapping)?;
    if doc.as_mapping().is_none() {
        return Err(ReadError::NotAMapping);
    }
    graph_from(&Node {
        yaml: &doc,
        path: String::new(),
    })
}

fn graph_from(root: &Node) -> R<Graph> {
    let version = u32_at(root, "version")?;
    if version != VERSION {
        return Err(ReadError::Version {
            found: version,
            expected: VERSION,
        });
    }

    let e = child(root, "execution")?;
    let execution = Execution {
        runtime: string_at(&e, "runtime")?,
        concurrency: u32_at(&e, "concurrency")?,
        timeout_sec: u32_at(&e, "timeoutSec")?,
    };

    let d = child(root, "defaults")?;
    let defaults = Defaults {
        provider: string_at(&d, "provider")?,
        model: string_at(&d, "model")?,
    };

    let o = child(root, "overlap")?;
    let overlap = Overlap {
        policy: enum_at(
            &o,
            "policy",
            &[
                ("queue", OverlapPolicy::Queue),
                ("dropNewest", OverlapPolicy::DropNewest),
                ("dropOldest", OverlapPolicy::DropOldest),
                ("coalesce", OverlapPolicy::Coalesce),
            ],
        )?,
        max_queue: u32_at(&o, "maxQueue")?,
        coalesce_ms: u32_at(&o, "coalesceMs")?,
        loop_parallel: u32_at(&o, "loopParallel")?,
    };

    let b = child(root, "between")?;
    let between = Between {
        keep_state: bool_at(&b, "keepState")?,
        restart_on_crash: bool_at(&b, "restartOnCrash")?,
    };

    let mut env = BTreeMap::new();
    if let Some(e) = optional_child(root, "env") {
        for (key, node) in mapping(&e)? {
            env.insert(key, string(&node)?);
        }
    }

    let mut blocks = Vec::new();
    for (i, item) in items(&child(root, "blocks")?)?.into_iter().enumerate() {
        blocks.push(block_from(&Node {
            yaml: item,
            path: format!("blocks[{i}]"),
        })?);
    }

    let mut frames = Vec::new();
    if let Some(f) = optional_child(root, "frames") {
        for (i, item) in items(&f)?.into_iter().enumerate() {
            frames.push(frame_from(&Node {
                yaml: item,
                path: format!("frames[{i}]"),
            })?);
        }
    }

    let mut wires = Vec::new();
    for (i, item) in items(&child(root, "wires")?)?.into_iter().enumerate() {
        let n = Node {
            yaml: item,
            path: format!("wires[{i}]"),
        };
        let from = string_at(&n, "from")?;
        let to = string_at(&n, "to")?;
        wires.push(Wire {
            id: string_at(&n, "id")?,
            from: endpoint(&from, &format!("{}.from", n.path))?,
            to: endpoint(&to, &format!("{}.to", n.path))?,
        });
    }

    let u = child(root, "ui")?;
    let v = numbers(&child(&u, "viewport")?, 3)?;
    let ui = Ui {
        viewport: Viewport {
            x: v[0],
            y: v[1],
            zoom: v[2],
        },
    };

    Ok(Graph {
        version,
        id: string_at(root, "id")?,
        name: string_at(root, "name")?,
        description: optional_string_at(root, "description")?,
        run_mode: enum_at(
            root,
            "runMode",
            &[
                ("once", RunMode::Once),
                ("live", RunMode::Live),
                ("schedule", RunMode::Schedule),
            ],
        )?,
        local_only: bool_at(root, "localOnly")?,
        execution,
        defaults,
        overlap,
        between,
        env,
        blocks,
        frames,
        wires,
        ui,
    })
}

fn block_from(n: &Node) -> R<Block> {
    let p = numbers(&child(n, "position")?, 2)?;
    let position = Position { x: p[0], y: p[1] };

    let size = match optional_child(n, "size") {
        Some(s) => {
            let v: Vec<f64> = items(&s)?
                .into_iter()
                .map(|y| {
                    number(&Node {
                        yaml: y,
                        path: s.path.clone(),
                    })
                })
                .collect::<R<_>>()?;
            match v.len() {
                1 => Some(Size { w: v[0], h: None }),
                2 => Some(Size {
                    w: v[0],
                    h: Some(v[1]),
                }),
                _ => {
                    return Err(ReadError::Invalid {
                        path: format!("{}.size", n.path),
                        message: "expected [width] or [width, height]".into(),
                    });
                }
            }
        }
        None => None,
    };

    let mut settings = BTreeMap::new();
    if let Some(s) = optional_child(n, "settings") {
        for (key, node) in mapping(&s)? {
            settings.insert(key, value_from(node.yaml)?);
        }
    }

    let mut ports = Vec::new();
    if let Some(p) = optional_child(n, "ports") {
        for (i, item) in items(&p)?.into_iter().enumerate() {
            let pn = Node {
                yaml: item,
                path: format!("{}.ports[{}]", n.path, i),
            };
            ports.push(Port {
                name: string_at(&pn, "name")?,
                port_type: port_type_at(&pn, "type")?,
                side: enum_at(&pn, "side", &[("in", Side::In), ("out", Side::Out)])?,
                optional: optional_bool_at(&pn, "optional")?.unwrap_or(false),
            });
        }
    }

    let source = match optional_child(n, "source") {
        Some(s) => Some(Source {
            mode: enum_at(
                &s,
                "mode",
                &[("inline", SourceMode::Inline), ("file", SourceMode::File)],
            )?,
            language: enum_at(
                &s,
                "language",
                &[
                    ("python", Language::Python),
                    ("typescript", Language::Typescript),
                    ("javascript", Language::Javascript),
                    ("shell", Language::Shell),
                ],
            )?,
            code: optional_string_at(&s, "code")?,
            path: optional_string_at(&s, "path")?,
        }),
        None => None,
    };

    Ok(Block {
        id: string_at(n, "id")?,
        kind: string_at(n, "kind")?,
        title: optional_string_at(n, "title")?,
        position,
        size,
        view: enum_at(
            n,
            "view",
            &[
                ("compact", View::Compact),
                ("summary", View::Summary),
                ("code", View::Code),
                ("stage", View::Stage),
            ],
        )?,
        settings,
        ports,
        source,
        disabled: optional_bool_at(n, "disabled")?.unwrap_or(false),
        breakpoint: optional_bool_at(n, "breakpoint")?.unwrap_or(false),
        frame: optional_string_at(n, "frame")?,
    })
}

fn frame_from(n: &Node) -> R<Frame> {
    let p = numbers(&child(n, "position")?, 2)?;
    let s = numbers(&child(n, "size")?, 2)?;
    let over = string_at(n, "over")?;
    let stop_when = optional_string_at(n, "stopWhen")?;
    Ok(Frame {
        id: string_at(n, "id")?,
        kind: enum_at(n, "kind", &[("loop", FrameKind::Loop)])?,
        position: Position { x: p[0], y: p[1] },
        size: Size {
            w: s[0],
            h: Some(s[1]),
        },
        over: endpoint(&over, &format!("{}.over", n.path))?,
        as_name: string_at(n, "as")?,
        parallel: u32_at(n, "parallel")?,
        max: u32_at(n, "max")?,
        stop_when: match stop_when {
            Some(s) => Some(endpoint(&s, &format!("{}.stopWhen", n.path))?),
            None => None,
        },
        continue_on_error: bool_at(n, "continueOnError")?,
    })
}

fn value_from(y: &Yaml) -> R<Setting> {
    if y.is_null() {
        return Ok(Setting::Null);
    }
    if let Some(b) = y.as_bool() {
        return Ok(Setting::Bool(b));
    }
    if let Some(i) = y.as_integer() {
        // Out of i32 range degrades to a float, which is the precision JSON
        // would have given it anyway; see the note on Setting.
        return Ok(match i32::try_from(i) {
            Ok(i) => Setting::Int(i),
            Err(_) => Setting::Float(i as f64),
        });
    }
    if let Some(f) = y.as_floating_point() {
        return Ok(Setting::Float(f));
    }
    if let Some(s) = y.as_str() {
        return Ok(Setting::String(s.to_owned()));
    }
    if let Some(seq) = y.as_sequence() {
        return Ok(Setting::List(seq.iter().map(value_from).collect::<R<_>>()?));
    }
    if let Some(map) = y.as_mapping() {
        let mut out = BTreeMap::new();
        for (k, v) in map {
            let key = k.as_str().ok_or(ReadError::Type {
                path: "settings".into(),
                expected: "a string key",
            })?;
            out.insert(key.to_owned(), value_from(v)?);
        }
        return Ok(Setting::Map(out));
    }
    Ok(Setting::Null)
}

/// `node.port`, where the node is a block or a loop frame. Port names never
/// contain a dot, so the split is unambiguous.
fn endpoint(s: &str, path: &str) -> R<Endpoint> {
    match s.split_once('.') {
        Some((node, port)) if !node.is_empty() && !port.is_empty() && !port.contains('.') => {
            Ok(Endpoint::new(node, port))
        }
        _ => Err(ReadError::Invalid {
            path: path.to_owned(),
            message: format!("expected `node.port`, found `{s}`"),
        }),
    }
}

// ---------------------------------------------------------------- accessors

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_owned()
    } else {
        format!("{path}.{key}")
    }
}

fn child<'a>(n: &'a Node<'a>, key: &str) -> R<Node<'a>> {
    n.yaml
        .as_mapping_get(key)
        .map(|y| Node {
            yaml: y,
            path: join(&n.path, key),
        })
        .ok_or_else(|| ReadError::Missing {
            path: join(&n.path, key),
        })
}

fn optional_child<'a>(n: &'a Node<'a>, key: &str) -> Option<Node<'a>> {
    n.yaml.as_mapping_get(key).map(|y| Node {
        yaml: y,
        path: join(&n.path, key),
    })
}

fn mapping<'a>(n: &'a Node<'a>) -> R<Vec<(String, Node<'a>)>> {
    let m = n.yaml.as_mapping().ok_or(ReadError::Type {
        path: n.path.clone(),
        expected: "a mapping",
    })?;
    let mut out = Vec::new();
    for (k, v) in m {
        let key = k.as_str().ok_or(ReadError::Type {
            path: n.path.clone(),
            expected: "a string key",
        })?;
        out.push((
            key.to_owned(),
            Node {
                yaml: v,
                path: join(&n.path, key),
            },
        ));
    }
    Ok(out)
}

fn items<'a>(n: &'a Node<'a>) -> R<Vec<&'a Yaml<'a>>> {
    n.yaml
        .as_sequence()
        .map(|s| s.iter().collect())
        .ok_or(ReadError::Type {
            path: n.path.clone(),
            expected: "a sequence",
        })
}

fn string(n: &Node) -> R<String> {
    n.yaml.as_str().map(str::to_owned).ok_or(ReadError::Type {
        path: n.path.clone(),
        expected: "a string",
    })
}

fn number(n: &Node) -> R<f64> {
    if let Some(i) = n.yaml.as_integer() {
        return Ok(i as f64);
    }
    n.yaml.as_floating_point().ok_or(ReadError::Type {
        path: n.path.clone(),
        expected: "a number",
    })
}

fn numbers(n: &Node, count: usize) -> R<Vec<f64>> {
    let seq = items(n)?;
    if seq.len() != count {
        return Err(ReadError::Invalid {
            path: n.path.clone(),
            message: format!("expected {count} numbers, found {}", seq.len()),
        });
    }
    seq.into_iter()
        .map(|y| {
            number(&Node {
                yaml: y,
                path: n.path.clone(),
            })
        })
        .collect()
}

fn string_at(n: &Node, key: &str) -> R<String> {
    string(&child(n, key)?)
}

fn optional_string_at(n: &Node, key: &str) -> R<Option<String>> {
    match optional_child(n, key) {
        Some(c) => Ok(Some(string(&c)?)),
        None => Ok(None),
    }
}

fn bool_at(n: &Node, key: &str) -> R<bool> {
    let c = child(n, key)?;
    c.yaml.as_bool().ok_or(ReadError::Type {
        path: c.path,
        expected: "true or false",
    })
}

fn optional_bool_at(n: &Node, key: &str) -> R<Option<bool>> {
    match optional_child(n, key) {
        Some(c) => Ok(Some(c.yaml.as_bool().ok_or(ReadError::Type {
            path: c.path,
            expected: "true or false",
        })?)),
        None => Ok(None),
    }
}

fn u32_at(n: &Node, key: &str) -> R<u32> {
    let c = child(n, key)?;
    let v = c.yaml.as_integer().ok_or(ReadError::Type {
        path: c.path.clone(),
        expected: "a whole number",
    })?;
    u32::try_from(v).map_err(|_| ReadError::Invalid {
        path: c.path,
        message: format!("{v} is out of range"),
    })
}

fn enum_at<T: Copy>(n: &Node, key: &str, options: &[(&str, T)]) -> R<T> {
    let c = child(n, key)?;
    let s = string(&c)?;
    options
        .iter()
        .find(|(name, _)| *name == s)
        .map(|(_, v)| *v)
        .ok_or_else(|| ReadError::Invalid {
            path: c.path.clone(),
            message: format!(
                "`{s}` is not one of: {}",
                options
                    .iter()
                    .map(|(k, _)| *k)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        })
}

fn port_type_at(n: &Node, key: &str) -> R<PortType> {
    let c = child(n, key)?;
    let s = string(&c)?;
    s.parse().map_err(|_| ReadError::Invalid {
        path: c.path,
        message: format!("`{s}` is not a port type"),
    })
}
