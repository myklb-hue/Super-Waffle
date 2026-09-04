//! Running a custom block (SPEC §10).
//!
//! The block is a function, so running it means calling that function with the
//! values on its input ports and the values of its settings, and taking what it
//! returns. The function's own source is loaded as written, unmodified — a
//! custom block runs the code the user is looking at, not a rewritten copy of
//! it, which is the only way a line number in a traceback means anything.
//!
//! # Where the result comes out
//!
//! Not stdout. A custom block's `print` is the user's, and it belongs in the
//! console beside every other line the graph produced. So the driver writes
//! the return value to a file whose path it is given, and stdout stays exactly
//! what the function chose to write. The alternative — a sentinel line in the
//! output — puts the engine in the position of hoping the user never prints
//! the sentinel.
//!
//! # Where the type names come from
//!
//! SPEC §10.1 writes annotations in the type system's own names: `Image`,
//! `Data`, `Text`. Python has never heard of them, and it evaluates
//! annotations when the `def` runs — so a block written exactly as the
//! specification writes it would fail on its first line with a `NameError`.
//! The runtime provides them, which is what makes them the *type system's*
//! names rather than a convention the parser happens to recognise.
//!
//! They are injected into `builtins` by a driver that then imports the user's
//! code as its own module. That is why there are two files rather than one:
//! a preamble prepended to the source would shift every line number in every
//! traceback by the length of the preamble, and a custom block's error has to
//! point at the line the editor is showing.

use super::blocks::{self, Outputs};
use super::value::Value;
use block_source::Interface;
use graph_format::{Block, Language, PortType, Side, SourceMode};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// A name nothing else is using.
///
/// The pid and the block id are not enough on their own: two runs of the same
/// graph, or two windows on one machine, would write each other's driver
/// halfway through reading it. The counter is what makes each call its own.
static NEXT: AtomicU64 = AtomicU64::new(0);

fn scratch(block: &Block, extension: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "cyberloom-{}-{}-{}.{extension}",
        std::process::id(),
        block.id,
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

/// Everything the driver needs to know, as JSON on the command line.
fn arguments(block: &Block, interface: &Interface, inputs: &Outputs) -> serde_json::Value {
    let mut args = serde_json::Map::new();

    // Inputs first: a wired port always wins over a setting of the same name,
    // because a wire is a live value and a setting is a default someone typed.
    for port in interface.ports.iter().filter(|p| p.side == Side::In) {
        if let Some(value) = inputs.get(&port.name) {
            args.insert(port.name.clone(), as_argument(port.port_type, value));
        }
    }

    for setting in &interface.settings {
        if args.contains_key(&setting.name) {
            continue;
        }
        // What the user set in the inspector, or the default the code wrote.
        match block.settings.get(&setting.name) {
            Some(stored) => {
                args.insert(setting.name.clone(), setting_as_json(stored));
            }
            None => {
                // The default is stored as source text, so it is read the way
                // the language would read it. A default this cannot parse is
                // left out entirely, and the function's own default applies —
                // which is exactly right, because the function has one.
                if let Some(value) = literal(&setting.default) {
                    args.insert(setting.name.clone(), value);
                }
            }
        }
    }
    serde_json::Value::Object(args)
}

/// A value on its way into a function, in the shape the port asked for.
///
/// The port's declared type decides the coercion, not the value's. `as_data`
/// reads text that happens to be JSON as the record it describes, which is
/// right for a `data` port and wrong everywhere else: a `Text` port handed
/// "12345678" must see a string, not a number that no longer has a length.
fn as_argument(port_type: PortType, value: &Value) -> serde_json::Value {
    match port_type {
        PortType::Text | PortType::File | PortType::Stream => {
            serde_json::Value::String(value.as_text())
        }
        PortType::Data => value.as_data(),
        // `any` means the grammar does not know, so nothing is assumed: the
        // value arrives as whatever it already is.
        _ => match value {
            Value::Text(s) => serde_json::Value::String(s.clone()),
            other => other.as_data(),
        },
    }
}

fn setting_as_json(value: &graph_format::Setting) -> serde_json::Value {
    match value {
        graph_format::Setting::Null => serde_json::Value::Null,
        graph_format::Setting::Bool(b) => serde_json::json!(b),
        graph_format::Setting::Int(i) => serde_json::json!(i),
        graph_format::Setting::Float(f) => serde_json::json!(f),
        graph_format::Setting::String(s) => serde_json::json!(s),
        graph_format::Setting::List(items) => {
            serde_json::Value::Array(items.iter().map(setting_as_json).collect())
        }
        graph_format::Setting::Map(map) => serde_json::Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), setting_as_json(v)))
                .collect(),
        ),
    }
}

/// A default written in source, as a JSON value.
fn literal(text: &str) -> Option<serde_json::Value> {
    let text = text.trim();
    match text {
        "True" | "true" => return Some(serde_json::json!(true)),
        "False" | "false" => return Some(serde_json::json!(false)),
        "None" | "null" | "nil" => return Some(serde_json::Value::Null),
        _ => {}
    }
    if let Ok(number) = text.parse::<f64>() {
        return Some(serde_json::json!(number));
    }
    if block_source::is_quoted(text) {
        return Some(serde_json::json!(block_source::unquote(text)));
    }
    // A dict, a list, a call: JSON reads the first two and not the third.
    serde_json::from_str(text).ok()
}

/// The driver that imports the user's code and calls the function.
///
/// The user's source is a module of its own, loaded from its own file, so a
/// traceback names that file and the line the editor shows. Everything the
/// engine needs is in here instead.
fn python_driver(function: &str) -> String {
    format!(
        r#"import builtins, importlib.util, json, sys

# The type system's names (SPEC §4.1), so an annotation written the way the
# specification writes it resolves. Subscriptable, so `Optional[Image]` and
# `list[Data]` work too.
class _Type:
    def __class_getitem__(cls, item):
        return cls

for _name in ("Text", "Data", "Image", "Audio", "File", "Stream", "Tools", "Memory", "Exec"):
    if not hasattr(builtins, _name):
        setattr(builtins, _name, type(_name, (_Type,), {{}}))

_spec = importlib.util.spec_from_file_location("cyberloom_block", sys.argv[1])
_module = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_module)

_fn = getattr(_module, {function:?}, None)
if _fn is None:
    raise NameError("the file has no function called {function}")

_out = _fn(**json.loads(sys.argv[2]))
with open(sys.argv[3], "w") as _f:
    json.dump(_out, _f, default=str)
"#
    )
}

/// The same, for Node. `--experimental-strip-types` is what makes a `.ts`
/// block and a `.js` block the same block kind (SPEC §10.5).
fn node_driver(function: &str) -> String {
    format!(
        r#"import {{ writeFileSync }} from 'node:fs';
import {{ pathToFileURL }} from 'node:url';

const module = await import(pathToFileURL(process.argv[2]).href);
const fn = module[{function:?}] ?? module.default;
if (typeof fn !== 'function') {{
  throw new Error('the file exports no function called {function}');
}}
const args = JSON.parse(process.argv[3]);
const out = await fn(...Object.values(args));
writeFileSync(process.argv[4], JSON.stringify(out ?? null));
"#
    )
}

/// Run the block's function and return what it produced, plus what it printed.
pub fn run(
    block: &Block,
    interface: &Interface,
    inputs: &Outputs,
    root: &Path,
) -> Result<(Outputs, blocks::Output), String> {
    let source_def = block
        .source
        .as_ref()
        .ok_or("this custom block has no code")?;

    if source_def.language == Language::Shell {
        // A shell block has no signature to call: the script is the body, and
        // its ports arrive as environment variables (SPEC §10.5).
        let code = read_source(source_def, root)?;
        return run_shell(block, interface, inputs, root, &code);
    }

    // Where the user's code lives while it runs.
    //
    // In file mode that is their own file, untouched, so a traceback names the
    // path they are editing. Inline code is written out under a name that says
    // which block it came from, which is the closest thing to a filename it has.
    let (module, temporary) = match source_def.mode {
        SourceMode::File => {
            let path = source_def
                .path
                .as_deref()
                .ok_or("the block is in file mode but names no file")?;
            let full = if Path::new(path).is_absolute() {
                Path::new(path).to_path_buf()
            } else {
                root.join(path)
            };
            if !full.exists() {
                return Err(format!("{} is not there", full.display()));
            }
            (full, false)
        }
        SourceMode::Inline => {
            let code = source_def
                .code
                .as_deref()
                .ok_or("the block is in inline mode but holds no code")?;
            let extension = match source_def.language {
                Language::Python => "py",
                Language::Typescript => "ts",
                _ => "mjs",
            };
            let path = scratch(block, extension);
            std::fs::write(&path, code)
                .map_err(|e| format!("could not write the block's code: {e}"))?;
            (path, true)
        }
    };

    let (program, driver, extension) = match source_def.language {
        Language::Python => (
            blocks::python_interpreter(block, root),
            python_driver(&interface.name),
            "py",
        ),
        _ => (PathBuf::from("node"), node_driver(&interface.name), "mjs"),
    };
    let driver_path = scratch(block, &format!("driver.{extension}"));
    std::fs::write(&driver_path, driver).map_err(|e| format!("could not write the driver: {e}"))?;

    let args = arguments(block, interface, inputs);
    let result_file = scratch(block, "json");

    let mut command = std::process::Command::new(&program);
    if source_def.language != Language::Python {
        // Node strips the types, which is what makes a `.ts` block and a `.js`
        // block the same block kind (SPEC §10.5).
        command
            .arg("--experimental-strip-types")
            .arg("--no-warnings");
    }
    let started = std::time::Instant::now();
    let output = command
        .arg(&driver_path)
        .arg(&module)
        .arg(args.to_string())
        .arg(&result_file)
        .current_dir(root)
        .output()
        .map_err(|e| format!("could not start {}: {e}", program.display()))?;

    let ran = blocks::Output {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        code: output.status.code().unwrap_or(-1),
        ms: started.elapsed().as_millis() as u32,
    };
    let _ = std::fs::remove_file(&driver_path);
    if temporary {
        let _ = std::fs::remove_file(&module);
    }

    if !ran.ok() {
        // The traceback is the whole message. A custom block that throws shows
        // the user their own error, not a translation of it.
        return Err(last_meaningful_line(&ran.stderr));
    }

    let produced = std::fs::read_to_string(&result_file).unwrap_or_else(|_| "null".into());
    let _ = std::fs::remove_file(&result_file);
    let value: serde_json::Value =
        serde_json::from_str(&produced).unwrap_or(serde_json::Value::Null);

    Ok((spread(interface, value), ran))
}

/// The block's code, wherever it lives.
fn read_source(source: &graph_format::Source, root: &Path) -> Result<String, String> {
    match source.mode {
        SourceMode::Inline => source
            .code
            .clone()
            .ok_or_else(|| "the block is in inline mode but holds no code".to_owned()),
        SourceMode::File => {
            let path = source
                .path
                .as_deref()
                .ok_or("the block is in file mode but names no file")?;
            let full = if Path::new(path).is_absolute() {
                Path::new(path).to_path_buf()
            } else {
                root.join(path)
            };
            std::fs::read_to_string(&full)
                .map_err(|e| format!("could not read {}: {e}", full.display()))
        }
    }
}

/// Run a shell block. Its ports arrive as environment variables and its result
/// is whatever it printed.
fn run_shell(
    block: &Block,
    interface: &Interface,
    inputs: &Outputs,
    root: &Path,
    code: &str,
) -> Result<(Outputs, blocks::Output), String> {
    let started = std::time::Instant::now();
    let mut command = std::process::Command::new("sh");
    command.arg("-c").arg(code).current_dir(root);
    for port in interface.ports.iter().filter(|p| p.side == Side::In) {
        if let Some(value) = inputs.get(&port.name) {
            command.env(port.name.to_uppercase(), value.as_text());
        }
    }
    for setting in &interface.settings {
        let value = match block.settings.get(&setting.name) {
            Some(stored) => setting_as_json(stored).to_string(),
            None => block_source::unquote(&setting.default),
        };
        command.env(setting.name.to_uppercase(), value.trim_matches('"'));
    }
    let output = command
        .output()
        .map_err(|e| format!("could not start a shell: {e}"))?;
    let ran = blocks::Output {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        code: output.status.code().unwrap_or(-1),
        ms: started.elapsed().as_millis() as u32,
    };
    if !ran.ok() {
        return Err(format!("exit {}: {}", ran.code, ran.stderr.trim()));
    }
    let mut out = Outputs::new();
    if let Some(port) = interface.ports.iter().find(|p| p.side == Side::Out) {
        out.insert(
            port.name.clone(),
            Value::Text(ran.stdout.trim_end().to_owned()),
        );
    }
    Ok((out, ran))
}

/// Put the return value on the block's output ports.
///
/// One port takes the whole value. Several — from a tuple annotation — take it
/// element by element, which is the only reading that makes a tuple return
/// mean anything.
fn spread(interface: &Interface, value: serde_json::Value) -> Outputs {
    let outs: Vec<&graph_format::Port> = interface
        .ports
        .iter()
        .filter(|p| p.side == Side::Out)
        .collect();
    let mut produced = Outputs::new();
    match outs.as_slice() {
        [] => {}
        [only] => {
            produced.insert(only.name.clone(), from_json(value));
        }
        many => {
            let items = match value {
                serde_json::Value::Array(items) => items,
                other => vec![other],
            };
            for (port, item) in many.iter().zip(items) {
                produced.insert(port.name.clone(), from_json(item));
            }
        }
    }
    produced
}

/// A JSON value as a runtime value. A string is text — anything else keeps its
/// structure, because a block that returned a record should hand on a record.
fn from_json(value: serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::String(s) => Value::Text(s),
        other => Value::Data(other),
    }
}

/// The last line of a traceback that says something, which is the line a
/// person reads first.
fn last_meaningful_line(stderr: &str) -> String {
    stderr
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("the block failed with no message")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use block_source::parse;
    use graph_format::{Position, Setting, Source, View};
    use std::collections::BTreeMap;

    fn custom(language: Language, code: &str, settings: &[(&str, Setting)]) -> Block {
        Block {
            id: "custom".into(),
            kind: "custom".into(),
            title: None,
            position: Position { x: 0.0, y: 0.0 },
            size: None,
            view: View::Summary,
            settings: settings
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
            ports: Vec::new(),
            source: Some(Source {
                mode: SourceMode::Inline,
                language,
                code: Some(code.to_owned()),
                path: None,
            }),
            disabled: false,
            breakpoint: false,
            frame: None,
        }
    }

    fn go(block: &Block, inputs: &[(&str, Value)]) -> Result<(Outputs, blocks::Output), String> {
        let source = block.source.as_ref().unwrap();
        let interface = parse(source.language, source.code.as_deref().unwrap())
            .unwrap()
            .remove(0);
        let mut map = Outputs::new();
        for (k, v) in inputs {
            map.insert((*k).to_owned(), v.clone());
        }
        run(block, &interface, &map, Path::new("/tmp"))
    }

    /// SPEC §13.4's block, run for real: an Image in, a threshold setting, a
    /// Data record out.
    #[test]
    fn a_python_block_runs_and_returns_a_record() {
        let block = custom(
            Language::Python,
            "def door_check(frame: Text, threshold: float = 0.6) -> Data:\n\
             \x20   score = len(frame) / 10\n\
             \x20   return {\"open\": score > threshold, \"score\": score}\n",
            &[],
        );
        let (out, _) = go(&block, &[("frame", Value::Text("12345678".into()))]).unwrap();
        assert_eq!(
            out["result"],
            Value::Data(serde_json::json!({ "open": true, "score": 0.8 }))
        );
    }

    /// The setting the user chose beats the default the code wrote.
    #[test]
    fn the_inspectors_value_wins_over_the_codes_default() {
        let block = custom(
            Language::Python,
            "def f(frame: Text, threshold: float = 0.6) -> Data:\n\
             \x20   return {\"t\": threshold}\n",
            &[("threshold", Setting::Float(0.9))],
        );
        let (out, _) = go(&block, &[("frame", Value::Text("x".into()))]).unwrap();
        assert_eq!(out["result"], Value::Data(serde_json::json!({ "t": 0.9 })));
    }

    /// A wired port beats a setting of the same name: a wire is a live value
    /// and a setting is something someone typed once.
    #[test]
    fn a_wire_wins_over_a_setting_of_the_same_name() {
        let mut block = custom(
            Language::Python,
            "def f(threshold: float = 0.6) -> Data:\n    return {\"t\": threshold}\n",
            &[("threshold", Setting::Float(0.1))],
        );
        // Make `threshold` a port as well, the way a reload would.
        block.ports = vec![graph_format::Port {
            name: "threshold".into(),
            port_type: graph_format::PortType::Any,
            side: Side::In,
            optional: false,
        }];
        let source = block.source.as_ref().unwrap();
        let mut interface = parse(source.language, source.code.as_deref().unwrap())
            .unwrap()
            .remove(0);
        interface.ports.push(block.ports[0].clone());
        let mut inputs = Outputs::new();
        inputs.insert("threshold".into(), Value::Data(serde_json::json!(0.42)));
        let (out, _) = run(&block, &interface, &inputs, Path::new("/tmp")).unwrap();
        assert_eq!(out["result"], Value::Data(serde_json::json!({ "t": 0.42 })));
    }

    /// What the function printed is the user's output and reaches the console;
    /// the return value comes back separately, so a `print` cannot corrupt it.
    #[test]
    fn printing_does_not_disturb_the_return_value() {
        let block = custom(
            Language::Python,
            "def f(x: Text) -> Data:\n\
             \x20   print('working')\n\
             \x20   print('{\"open\": false}')\n\
             \x20   return {\"open\": True}\n",
            &[],
        );
        let (out, ran) = go(&block, &[("x", Value::Text("a".into()))]).unwrap();
        assert!(ran.stdout.contains("working"));
        assert_eq!(
            out["result"],
            Value::Data(serde_json::json!({ "open": true }))
        );
    }

    /// A tuple return spreads across the ports it made.
    #[test]
    fn a_tuple_return_fills_every_output_port() {
        let block = custom(
            Language::Python,
            "def f(x: Text) -> tuple[Data, Text]:\n    return ({\"n\": 1}, \"done\")\n",
            &[],
        );
        let (out, _) = go(&block, &[("x", Value::Text("a".into()))]).unwrap();
        assert_eq!(out["result1"], Value::Data(serde_json::json!({ "n": 1 })));
        assert_eq!(out["result2"], Value::Text("done".into()));
    }

    /// A block that throws shows the user their own error.
    #[test]
    fn an_error_comes_back_as_the_message_python_wrote() {
        let block = custom(
            Language::Python,
            "def f(x: Text) -> Data:\n    raise ValueError('no door in frame')\n",
            &[],
        );
        let error = go(&block, &[("x", Value::Text("a".into()))]).unwrap_err();
        assert!(error.contains("no door in frame"), "{error}");
    }

    /// A shell block takes its ports as environment variables.
    #[test]
    fn a_shell_block_reads_its_ports_from_the_environment() {
        let block = custom(
            Language::Shell,
            "# @block greet\n# @in name: Text\n# @out result: Text\n# @set greeting: str = \"Hello\"\necho \"$GREETING, $NAME\"\n",
            &[],
        );
        let (out, _) = go(&block, &[("name", Value::Text("world".into()))]).unwrap();
        assert_eq!(out["result"], Value::Text("Hello, world".into()));
    }

    #[test]
    fn a_setting_map_survives_the_trip() {
        let mut settings = BTreeMap::new();
        settings.insert(
            "opts".to_owned(),
            Setting::List(vec![Setting::Int(1), Setting::Int(2)]),
        );
        let block = Block {
            settings,
            ..custom(
                Language::Python,
                "def f(x: Text, opts: Data = None) -> Data:\n    return {\"n\": len(opts or [])}\n",
                &[],
            )
        };
        let (out, _) = go(&block, &[("x", Value::Text("a".into()))]).unwrap();
        assert_eq!(out["result"], Value::Data(serde_json::json!({ "n": 2 })));
    }
}
