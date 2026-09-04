//! What each kind of block actually does.
//!
//! Two shapes live here, and they are not the same thing (see `plan`):
//!
//! - a **step** runs once, in order, taking values from its wired inputs;
//! - a **capability** answers calls, and only when a holder makes one.
//!
//! A Terminal is both, depending on how it was wired, and the two paths share
//! the same execution: `terminal.run("cargo build")` and a Terminal step with
//! `command: cargo build` run the same command the same way. Anything else and
//! the model would be running a different shell from the user.

use super::model::ToolDef;
use super::value::Value;
use graph_format::Block;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// What running something produced.
#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
    pub ms: u32,
}

impl Output {
    /// What a model reads back from a tool call.
    ///
    /// The exit code is included in words rather than left to be inferred from
    /// an empty stdout, because the whole point of the triage example is that
    /// the model reads a failure and reasons about it (SPEC §13.1). A tool that
    /// returned only stdout would hand it an empty string and nothing to go on.
    pub fn as_tool_result(&self) -> String {
        let mut out = format!("exit {}", self.code);
        if !self.stdout.trim().is_empty() {
            out.push_str("\nstdout:\n");
            out.push_str(self.stdout.trim_end());
        }
        if !self.stderr.trim().is_empty() {
            out.push_str("\nstderr:\n");
            out.push_str(self.stderr.trim_end());
        }
        out
    }

    /// The line the block shows inline while the run is in flight (SPEC §3.2).
    pub fn figure(&self) -> String {
        let lines = self.stdout.lines().count() + self.stderr.lines().count();
        if self.code == 0 {
            format!("exit 0 · {lines} line{}", if lines == 1 { "" } else { "s" })
        } else {
            format!("exit {}", self.code)
        }
    }

    pub fn ok(&self) -> bool {
        self.code == 0
    }
}

/// What the kind says this setting falls back to, if anything.
///
/// The file holds only what the user chose; everything else comes from the
/// catalogue. Reading the two in one place is what stops the engine and the
/// inspector disagreeing about what a block is going to do.
fn declared(block: &Block, name: &str) -> Option<&'static str> {
    block_kinds::kind(&block.kind)?.setting(name)?.default
}

/// A setting as a string: what the user chose, or what the kind falls back to.
///
/// The distinction matters: a setting the user left empty and a setting the
/// user cleared are the same thing, and both mean "fall back", never "use an
/// empty string".
pub fn setting<'a>(block: &'a Block, name: &str) -> Option<&'a str> {
    match block.settings.get(name) {
        Some(graph_format::Setting::String(s)) if !s.trim().is_empty() => Some(s),
        _ => declared(block, name),
    }
}

/// A switch, which always has a side (SPEC §7): the user's, or the kind's.
///
/// This used to read `false` for anything the file did not mention, which made
/// every safety switch off unless someone had turned it on — the opposite of
/// what SPEC §12.2 says about shell commands and physical actions. A
/// `warnBefore` nobody has touched now warns, because that is what the
/// catalogue declares.
pub fn flag(block: &Block, name: &str) -> bool {
    match block.settings.get(name) {
        Some(graph_format::Setting::Bool(b)) => *b,
        _ => declared(block, name) == Some("true"),
    }
}

pub fn number(block: &Block, name: &str) -> Option<f64> {
    match block.settings.get(name) {
        Some(graph_format::Setting::Int(i)) => Some(f64::from(*i)),
        Some(graph_format::Setting::Float(f)) => Some(*f),
        _ => declared(block, name).and_then(|d| d.parse().ok()),
    }
}

/// Run a shell command.
///
/// Through `sh -c` rather than by splitting the string: the setting holds a
/// command line, `cargo build 2>&1 | tail` is a reasonable thing to type into
/// it, and a hand-rolled split would get quoting wrong in a way that fails only
/// on the commands that matter.
pub fn shell(command: &str, cwd: &Path, timeout_hint: u32) -> Result<Output, String> {
    let _ = timeout_hint;
    let started = Instant::now();
    let result = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("could not start a shell: {e}"))?;
    Ok(Output {
        stdout: String::from_utf8_lossy(&result.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
        // A process killed by a signal has no exit code. -1 stands for "did not
        // exit on its own", which is what the console should say rather than
        // pretending it succeeded.
        code: result.status.code().unwrap_or(-1),
        ms: started.elapsed().as_millis() as u32,
    })
}

/// The interpreter a Python block should use.
///
/// A `venv` setting names an environment folder, and the interpreter inside it
/// is what makes the workspace's packages importable. Without one this falls
/// back to `python3` on the path, which is the honest default: the block says
/// "in the workspace environment" (SPEC §6.3), and when there is no environment
/// configured the workspace's environment is the one the user is already in.
pub fn python_interpreter(block: &Block, root: &Path) -> PathBuf {
    match setting(block, "venv") {
        Some(venv) => {
            let base = if Path::new(venv).is_absolute() {
                PathBuf::from(venv)
            } else {
                root.join(venv)
            };
            base.join("bin").join("python3")
        }
        None => PathBuf::from("python3"),
    }
}

/// Run Python source.
pub fn python(block: &Block, code: &str, root: &Path) -> Result<Output, String> {
    let started = Instant::now();
    let result = Command::new(python_interpreter(block, root))
        .arg("-c")
        .arg(code)
        .current_dir(root)
        .output()
        .map_err(|e| format!("could not start python: {e}"))?;
    Ok(Output {
        stdout: String::from_utf8_lossy(&result.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
        code: result.status.code().unwrap_or(-1),
        ms: started.elapsed().as_millis() as u32,
    })
}

/// The tools a block offers a model, named as SPEC §6 names them.
///
/// Returns nothing for a kind that has no callable form, which is how a
/// capability the engine does not yet implement stays out of a model's tool
/// list rather than being offered and then failing when called.
pub fn tools_of(block: &Block) -> Vec<ToolDef> {
    let id = &block.id;
    match block.kind.as_str() {
        "terminal" => vec![ToolDef {
            name: format!("{id}.run"),
            description: match setting(block, "command") {
                Some(default) => format!(
                    "Run a shell command and return its output and exit code. \
                     With no command given, runs `{default}`."
                ),
                None => "Run a shell command and return its output and exit code.".into(),
            },
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command line to run."
                    }
                },
                "required": ["command"]
            }),
        }],
        "python" => vec![ToolDef {
            name: format!("{id}.exec"),
            description: "Run Python in the workspace environment and return \
                          what it printed."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "The Python source to run." }
                },
                "required": ["code"]
            }),
        }],
        _ => Vec::new(),
    }
}

/// What a step produces on each of its output ports.
pub type Outputs = BTreeMap<String, Value>;

/// Run one step that needs nothing but its settings and its inputs.
///
/// The runtimes and the model are not here: they stream, they can warn, and
/// they need the runner's event sink, so they live in `runner`. What is here is
/// every block whose whole behaviour is a function of what it was given.
pub fn pure_step(block: &Block, inputs: &Outputs) -> Result<Outputs, String> {
    let mut out = Outputs::new();
    match block.kind.as_str() {
        "input" => {
            // The setting is the value. An Input with nothing in it produces
            // an empty string rather than failing: an empty prompt is a
            // reasonable thing to run while building a graph.
            out.insert(
                "value".into(),
                Value::Text(setting(block, "value").unwrap_or_default().to_owned()),
            );
        }
        "output" => {
            // An Output is a name for a result. It produces nothing; the run's
            // results are collected from what arrived here.
            let _ = inputs;
        }
        "convert" => {
            let value = inputs.get("value").cloned().unwrap_or(Value::Null);
            let to = setting(block, "to").unwrap_or("text");
            out.insert(
                "value".into(),
                match to {
                    "text" => Value::Text(value.as_text()),
                    "data" => Value::Data(value.as_data()),
                    "file" => Value::File(value.as_text()),
                    // image and audio cannot be conjured from a value that is
                    // not already one; saying so beats producing a broken path.
                    other => {
                        return Err(format!(
                            "converting to {other} needs a decoder this engine does not have yet"
                        ));
                    }
                },
            );
        }
        "branch" => {
            // A Branch chooses which of two control-flow paths continues
            // (SPEC §6.8). The value it was given is what the condition reads.
            let value = inputs.get("value").cloned().unwrap_or(Value::Null);
            let taken = if condition_holds(setting(block, "condition"), &value) {
                "a"
            } else {
                "b"
            };
            // Only the taken port produces anything. The other says nothing,
            // and the runner reads that silence as "not on this path" —
            // which is what makes a branch a branch rather than a fork.
            out.insert(taken.into(), Value::Null);
        }
        "merge" => {
            // Whatever arrived. Fan-in on one port already resolves to the
            // last writer (see `Runner::gather`), so a Merge is that made
            // explicit on the canvas.
            out.insert(
                "value".into(),
                inputs.values().next().cloned().unwrap_or(Value::Null),
            );
        }
        "variable" => {
            let value = inputs.get("value").cloned().unwrap_or(Value::Null);
            out.insert("value".into(), value);
        }
        other => return Err(format!("`{other}` is not a kind this engine can run yet")),
    }
    Ok(out)
}

/// Whether a Branch's condition holds.
///
/// Deliberately small: `field == "value"`, `field != "value"`, a comparison
/// against a number, or a bare field name read for truth. A Branch decides
/// between two paths on the canvas, and a graph whose logic has outgrown that
/// wants a custom block — where it would be written in a language with a
/// debugger rather than in a text field.
///
/// An empty condition is *true*: a Branch nobody has configured takes its
/// first path rather than silently taking neither.
pub fn condition_holds(condition: Option<&str>, value: &Value) -> bool {
    let Some(text) = condition.map(str::trim).filter(|c| !c.is_empty()) else {
        return true;
    };

    for (op, test) in [
        ("==", 0u8),
        ("!=", 1),
        (">=", 2),
        ("<=", 3),
        (">", 4),
        ("<", 5),
    ] {
        let Some((left, right)) = text.split_once(op) else {
            continue;
        };
        let got = field(value, left.trim());
        let want = block_source::unquote(right.trim());
        return match test {
            0 => got == want,
            1 => got != want,
            _ => {
                let (Ok(a), Ok(b)) = (got.parse::<f64>(), want.parse::<f64>()) else {
                    return false;
                };
                match test {
                    2 => a >= b,
                    3 => a <= b,
                    4 => a > b,
                    _ => a < b,
                }
            }
        };
    }

    // No operator: the named field read for truth.
    let got = field(value, text);
    !matches!(got.as_str(), "" | "false" | "0" | "null" | "none")
}

/// A named field of the value, or the value itself when the name does not
/// match one — so `label == "urgent"` works on a record *and* on the bare text
/// a classifier returns.
fn field(value: &Value, name: &str) -> String {
    if let Value::Data(serde_json::Value::Object(map)) = value
        && let Some(found) = map.get(name)
    {
        return match found {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
    }
    value.as_text().trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(kind: &str, settings: &[(&str, graph_format::Setting)]) -> Block {
        Block {
            id: kind.into(),
            kind: kind.into(),
            title: None,
            position: graph_format::Position { x: 0.0, y: 0.0 },
            size: None,
            view: graph_format::View::Summary,
            settings: settings
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
            ports: Vec::new(),
            source: None,
            disabled: false,
            breakpoint: false,
            frame: None,
        }
    }

    #[test]
    fn a_shell_command_reports_its_exit_code_and_both_streams() {
        let out = shell("echo out; echo err >&2; exit 3", Path::new("/"), 30).unwrap();
        assert_eq!(out.code, 3);
        assert_eq!(out.stdout.trim(), "out");
        assert_eq!(out.stderr.trim(), "err");
        assert!(!out.ok());
    }

    /// The command line is handed to a shell whole, so pipes and redirection
    /// work rather than being mangled by a naive split.
    #[test]
    fn a_pipeline_is_a_command() {
        let out = shell("printf 'a\\nb\\nc\\n' | wc -l", Path::new("/"), 30).unwrap();
        assert_eq!(out.stdout.trim(), "3");
    }

    /// What the model reads is the whole picture: a failing build with an empty
    /// stdout still says what happened.
    #[test]
    fn a_tool_result_leads_with_the_exit_code() {
        let out = Output {
            stdout: String::new(),
            stderr: "error: linking with `cc` failed".into(),
            code: 101,
            ms: 4,
        };
        let text = out.as_tool_result();
        assert!(text.starts_with("exit 101"));
        assert!(text.contains("linking with"));
        assert!(
            !text.contains("stdout"),
            "an empty stream is left out: {text}"
        );
        assert_eq!(out.figure(), "exit 101");
    }

    #[test]
    fn the_terminal_offers_its_setting_as_the_default_in_the_description() {
        let tools = tools_of(&block(
            "terminal",
            &[(
                "command",
                graph_format::Setting::String("cargo build".into()),
            )],
        ));
        assert_eq!(tools[0].name, "terminal.run");
        assert!(tools[0].description.contains("cargo build"));
    }

    /// A kind with no callable form is not offered. A model that was told about
    /// a tool the engine cannot run would call it and get an error it could do
    /// nothing about.
    #[test]
    fn a_kind_with_no_tool_offers_none() {
        assert!(tools_of(&block("webcam", &[])).is_empty());
    }

    #[test]
    fn an_input_is_its_setting() {
        let out = pure_step(
            &block(
                "input",
                &[("value", graph_format::Setting::String("hello".into()))],
            ),
            &Outputs::new(),
        )
        .unwrap();
        assert_eq!(out["value"], Value::Text("hello".into()));
    }

    #[test]
    fn convert_makes_text_into_data_and_back() {
        let mut inputs = Outputs::new();
        inputs.insert("value".into(), Value::Text(r#"{"exit":101}"#.into()));
        let out = pure_step(
            &block(
                "convert",
                &[("to", graph_format::Setting::String("data".into()))],
            ),
            &inputs,
        )
        .unwrap();
        assert_eq!(
            out["value"],
            Value::Data(serde_json::json!({ "exit": 101 }))
        );
    }

    /// A conversion the engine cannot do says so instead of producing a value
    /// that is not one.
    #[test]
    fn converting_to_an_image_is_refused_in_words() {
        let mut inputs = Outputs::new();
        inputs.insert("value".into(), Value::Text("hello".into()));
        let err = pure_step(
            &block(
                "convert",
                &[("to", graph_format::Setting::String("image".into()))],
            ),
            &inputs,
        )
        .unwrap_err();
        assert!(err.contains("decoder"), "{err}");
    }

    /// SPEC §13.2's own condition, on the bare label a classifier returns and
    /// on a record with a `label` field. Both are the same question.
    #[test]
    fn a_branch_reads_its_condition_against_text_or_a_record() {
        let urgent = Some("label == \"urgent\"");
        assert!(condition_holds(urgent, &Value::Text("urgent".into())));
        assert!(!condition_holds(urgent, &Value::Text("routine".into())));
        assert!(condition_holds(
            urgent,
            &Value::Data(serde_json::json!({ "label": "urgent", "score": 0.9 }))
        ));
        assert!(!condition_holds(
            urgent,
            &Value::Data(serde_json::json!({ "label": "noise" }))
        ));
    }

    #[test]
    fn a_branch_compares_numbers_and_reads_truth() {
        let record = Value::Data(serde_json::json!({ "score": 0.9, "open": true }));
        assert!(condition_holds(Some("score > 0.5"), &record));
        assert!(!condition_holds(Some("score > 0.95"), &record));
        assert!(condition_holds(Some("open"), &record));
        assert!(!condition_holds(
            Some("open"),
            &Value::Data(serde_json::json!({ "open": false }))
        ));
    }

    /// A Branch nobody configured takes its first path rather than neither.
    #[test]
    fn an_empty_condition_holds() {
        assert!(condition_holds(None, &Value::Null));
        assert!(condition_holds(
            Some("   "),
            &Value::Text("anything".into())
        ));
    }

    #[test]
    fn a_branch_produces_only_the_port_it_took() {
        let mut inputs = Outputs::new();
        inputs.insert("value".into(), Value::Text("urgent".into()));
        let taken = pure_step(
            &block(
                "branch",
                &[(
                    "condition",
                    graph_format::Setting::String("label == \"urgent\"".into()),
                )],
            ),
            &inputs,
        )
        .unwrap();
        assert_eq!(taken.keys().collect::<Vec<_>>(), ["a"]);

        inputs.insert("value".into(), Value::Text("noise".into()));
        let other = pure_step(
            &block(
                "branch",
                &[(
                    "condition",
                    graph_format::Setting::String("label == \"urgent\"".into()),
                )],
            ),
            &inputs,
        )
        .unwrap();
        assert_eq!(other.keys().collect::<Vec<_>>(), ["b"]);
    }

    /// A kind this slice has not reached is named, not skipped silently.
    #[test]
    fn an_unimplemented_kind_says_which_one() {
        let err = pure_step(&block("webcam", &[]), &Outputs::new()).unwrap_err();
        assert!(err.contains("webcam"), "{err}");
    }

    /// The half of a default that is not cosmetic.
    ///
    /// Before the catalogue carried defaults, `flag` read `false` for anything
    /// the file did not mention — so a Terminal dropped on a canvas ran shell
    /// commands without asking, which is the opposite of SPEC §12.2. A file
    /// that says nothing now gets the catalogue's answer.
    #[test]
    fn a_safety_switch_is_on_until_someone_turns_it_off() {
        let bare = block("terminal", &[]);
        assert!(flag(&bare, "warnBefore"), "a Terminal warns by default");

        let off = block(
            "terminal",
            &[("warnBefore", graph_format::Setting::Bool(false))],
        );
        assert!(!flag(&off, "warnBefore"), "and the user can turn it off");

        // A switch the catalogue declares off stays off: §12.3 says frames are
        // not recorded unless someone turns recording on.
        assert!(!flag(&block("webcam", &[]), "store"));
    }

    /// A setting nobody chose reads as what the kind says it is, and a setting
    /// the kind says nothing about reads as nothing.
    #[test]
    fn a_declared_default_fills_in_and_an_undeclared_one_does_not() {
        let llm = block("llm", &[]);
        assert_eq!(setting(&llm, "role"), Some("assistant"));
        // A temperature has no default on purpose: the request leaves it out
        // and the provider's own applies.
        assert_eq!(number(&llm, "temperature"), None);
        // SPEC §8.3: two items in parallel per loop frame.
        assert_eq!(number(&block("loop", &[]), "parallel"), Some(2.0));
    }

    #[test]
    fn a_venv_setting_picks_the_interpreter_inside_it() {
        let b = block(
            "python",
            &[("venv", graph_format::Setting::String(".venv".into()))],
        );
        assert_eq!(
            python_interpreter(&b, Path::new("/w")),
            PathBuf::from("/w/.venv/bin/python3")
        );
        // With no environment configured, the one the user is already in.
        assert_eq!(
            python_interpreter(&block("python", &[]), Path::new("/w")),
            PathBuf::from("python3")
        );
    }
}
