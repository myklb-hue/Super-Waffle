//! What a workspace knows about the machine it is on (SPEC §3, `/w/settings`).
//!
//! A workspace is a folder, and its settings are a file in that folder. Which
//! means they travel with it, they are readable, they are diffable, and a
//! workspace copied to another machine arrives with its intentions intact and
//! its detections wrong — which is the right way round, because the detections
//! are cheap to redo and the intentions are not.
//!
//! Two halves, kept apart on purpose. `Settings` is what the user chose;
//! `Probe` is what is actually installed. The settings screen shows both, and
//! never writes the second into the first: an engine that remembered where
//! Python was in March is an engine that is wrong in April.

use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::Path;

/// What the user chose, stored in `workspace.yaml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSettings {
    /// Where Python is, or none to use whatever `python3` resolves to.
    pub python: Option<String>,
    /// Where model weights live, or none for the default beside the workspace.
    pub models: Option<String>,
    /// The Ollama endpoint, or none for the local default.
    pub ollama: Option<String>,
    /// What a new graph starts with (SPEC §15.4). On, always, unless the user
    /// has said otherwise for this workspace.
    pub local_only_default: bool,
    /// The model a new graph starts with.
    pub model: Option<String>,
}

impl Default for WorkspaceSettings {
    fn default() -> Self {
        Self {
            python: None,
            models: None,
            ollama: None,
            local_only_default: true,
            model: None,
        }
    }
}

const FILE: &str = "workspace.yaml";

impl WorkspaceSettings {
    /// Read the workspace's settings, or the defaults when it has none.
    ///
    /// A workspace with no settings file is not an error and never becomes one:
    /// the file appears the first time something is changed, so a folder full
    /// of graphs is a workspace without ceremony.
    pub fn read(root: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(root.join(FILE)) else {
            return Self::default();
        };
        let mut settings = Self::default();
        for raw in text.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
            let some = (!value.is_empty()).then(|| value.to_owned());
            match key.trim() {
                "python" => settings.python = some,
                "models" => settings.models = some,
                "ollama" => settings.ollama = some,
                "model" => settings.model = some,
                "localOnlyDefault" => settings.local_only_default = value != "false",
                _ => {}
            }
        }
        settings
    }

    pub fn write(&self, root: &Path) -> Result<(), String> {
        let mut out = String::from(
            "# Cyberloom workspace settings.\n\
             #\n\
             # What you chose, not what is installed: the engine detects Python,\n\
             # ffmpeg and Ollama every time it starts, and never writes what it\n\
             # found in here. A path below is an override for this workspace.\n",
        );
        let line = |out: &mut String, key: &str, value: &Option<String>| {
            if let Some(value) = value {
                out.push_str(&format!("{key}: {value}\n"));
            }
        };
        line(&mut out, "python", &self.python);
        line(&mut out, "models", &self.models);
        line(&mut out, "ollama", &self.ollama);
        line(&mut out, "model", &self.model);
        out.push_str(&format!("localOnlyDefault: {}\n", self.local_only_default));
        std::fs::write(root.join(FILE), out).map_err(|e| format!("could not write {FILE}: {e}"))
    }
}

/// What is actually on this machine (SPEC §15.13, and slice 12's first run).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    pub python: Found,
    pub ffmpeg: Found,
    pub ollama: Found,
    /// Where perception weights would go, and whether anything is there.
    pub models: Found,
}

/// One thing looked for, and what was found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub name: String,
    /// Whether it is there and usable.
    pub ok: bool,
    /// The version, the path, or why not — in words a person can act on.
    pub detail: String,
    /// What to do about it, when there is something to do.
    pub fix: Option<String>,
}

impl Probe {
    /// Look for everything, now.
    ///
    /// Every time the screen is opened rather than once at startup: a person
    /// who has just installed ffmpeg in another window should see that by
    /// coming back to this screen, not by restarting the application.
    pub fn now(settings: &WorkspaceSettings, root: &Path) -> Self {
        Self {
            python: version(
                settings.python.as_deref().unwrap_or("python3"),
                "Python",
                Some("Install python3, or point at an interpreter in Settings."),
            ),
            ffmpeg: version(
                "ffmpeg",
                "ffmpeg",
                Some("Install ffmpeg. Without it a Webcam, Microphone or Speaker cannot run."),
            ),
            ollama: ollama(settings.ollama.as_deref()),
            models: models(settings.models.as_deref(), root),
        }
    }
}

/// Ask a program what version it is, and read the first line it says.
///
/// Both spellings, because `--version` is not universal: ffmpeg wants
/// `-version` and answers the long form with an error. Trying one and
/// reporting "not installed" would have said something false about a program
/// sitting right there on the path.
fn version(program: &str, name: &str, fix: Option<&str>) -> Found {
    for flag in ["--version", "-version"] {
        let Ok(out) = std::process::Command::new(program).arg(flag).output() else {
            // The program is not there at all; a second flag will not help.
            break;
        };
        if !out.status.success() && out.stdout.is_empty() {
            continue;
        }
        let said = String::from_utf8_lossy(if out.stdout.is_empty() {
            &out.stderr
        } else {
            &out.stdout
        });
        return Found {
            name: name.to_owned(),
            ok: true,
            detail: said.lines().next().unwrap_or("").trim().to_owned(),
            fix: None,
        };
    }
    Found {
        name: name.to_owned(),
        ok: false,
        detail: format!("`{program}` is not on the path"),
        fix: fix.map(str::to_owned),
    }
}

fn ollama(endpoint: Option<&str>) -> Found {
    let base = endpoint.unwrap_or("http://127.0.0.1:11434");
    let asked = ureq::get(&format!("{base}/api/tags"))
        .config()
        .timeout_global(Some(std::time::Duration::from_millis(700)))
        .build()
        .call();
    match asked {
        Ok(mut answer) => {
            let models = answer
                .body_mut()
                .read_json::<serde_json::Value>()
                .ok()
                .and_then(|body| Some(body.get("models")?.as_array()?.len()))
                .unwrap_or(0);
            Found {
                name: "Ollama".to_owned(),
                ok: true,
                detail: format!(
                    "{base} · {models} model{}",
                    if models == 1 { "" } else { "s" }
                ),
                fix: (models == 0).then(|| "Pull a model: `ollama pull llama3.2:3b`.".to_owned()),
            }
        }
        Err(_) => Found {
            name: "Ollama".to_owned(),
            ok: false,
            detail: format!("nothing is answering at {base}"),
            fix: Some("Start it with `ollama serve`, or set another endpoint here.".to_owned()),
        },
    }
}

fn models(chosen: Option<&str>, root: &Path) -> Found {
    let folder = match chosen {
        Some(named) if Path::new(named).is_absolute() => Path::new(named).to_path_buf(),
        Some(named) => root.join(named),
        None => root.join("models"),
    };
    let weights = std::fs::read_dir(&folder)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|x| x == "onnx" || x == "gguf" || x == "bin")
                })
                .count()
        })
        .unwrap_or(0);
    Found {
        name: "Models".to_owned(),
        ok: weights > 0,
        detail: format!(
            "{} · {weights} weight{}",
            folder.display(),
            if weights == 1 { "" } else { "s" }
        ),
        fix: (weights == 0).then(|| {
            "Perception blocks need weights here. Downloading them is explicit and \
             resumable, and offline is a supported state."
                .to_owned()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cyberloom-settings-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A folder of graphs is a workspace without ceremony.
    #[test]
    fn a_workspace_with_no_settings_file_has_the_defaults() {
        let dir = scratch("none");
        let settings = WorkspaceSettings::read(&dir);
        assert_eq!(settings, WorkspaceSettings::default());
        assert!(settings.local_only_default, "§15.4: on by default");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn what_is_written_is_what_is_read_back() {
        let dir = scratch("roundtrip");
        let settings = WorkspaceSettings {
            python: Some("/usr/bin/python3.12".into()),
            models: Some("weights".into()),
            ollama: None,
            local_only_default: false,
            model: Some("llama3.2:3b".into()),
        };
        settings.write(&dir).unwrap();
        assert_eq!(WorkspaceSettings::read(&dir), settings);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The file says what it is for, so someone reading the folder is not left
    /// guessing whether it is generated.
    #[test]
    fn the_file_explains_itself() {
        let dir = scratch("comment");
        WorkspaceSettings::default().write(&dir).unwrap();
        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(text.starts_with('#'), "{text}");
        assert!(text.contains("not what is installed"), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The probe is about this machine, and this machine has ffmpeg and python
    /// because slice 7 installed them.
    #[test]
    fn the_probe_finds_what_is_here_and_says_so_about_what_is_not() {
        let dir = scratch("probe");
        let probe = Probe::now(&WorkspaceSettings::default(), &dir);
        assert!(probe.python.ok, "{:?}", probe.python);
        assert!(probe.ffmpeg.ok, "{:?}", probe.ffmpeg);
        // No weights in a folder that was made a moment ago, and the fix says
        // what to do rather than only that something is missing.
        assert!(!probe.models.ok);
        assert!(probe.models.fix.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// ffmpeg answers `-version` and refuses `--version`. Asking only the long
    /// form said "not on the path" about a program sitting on the path.
    #[test]
    fn a_program_that_spells_its_version_flag_differently_is_still_found() {
        let found = version("ffmpeg", "ffmpeg", None);
        assert!(found.ok, "{found:?}");
        assert!(found.detail.contains("ffmpeg version"), "{found:?}");
    }

    #[test]
    fn something_that_is_not_installed_is_reported_with_a_fix() {
        let missing = version("definitely-not-a-program", "Nothing", Some("Install it."));
        assert!(!missing.ok);
        assert_eq!(missing.fix.as_deref(), Some("Install it."));
    }
}
