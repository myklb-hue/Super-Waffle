//! Rigs: what the Avatar is wearing, and what it can therefore do (SPEC §11).
//!
//! A rig is a folder — a `rig.yaml` naming what it supports, and one SVG per
//! expression. That is the whole format. It is content rather than code (§11.1):
//! adding a rig is copying a folder, not writing a block, and nothing in here
//! executes anything a rig author wrote.
//!
//! The important consequence is §11.2: the `face.express` vocabulary is
//! *generated* from the rig. The model can only ask for expressions that exist,
//! which is the custom-block rule (§10.1) applied to animation — the interface
//! is derived, never typed by hand. A Pixel rig has no `sleepy` because an
//! 8 × 8 matrix has nowhere to put one, and a model driving a Pixel is never
//! offered it.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// One aesthetic, and the animations it supports.
#[derive(Debug, Clone, PartialEq)]
pub struct Rig {
    /// The folder name, which is what a graph stores.
    pub id: String,
    pub name: String,
    pub description: String,
    /// In manifest order, which is the order the panel lists them in.
    pub expressions: Vec<String>,
    pub gestures: Vec<String>,
    /// Whether `face.look` means anything on this rig.
    pub gaze: bool,
    pub idle: Idle,
    /// Where the states are, so the shell can be told which file to draw.
    pub folder: PathBuf,
}

/// What the block does between turns, so the avatar is alive (SPEC §11.4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Idle {
    pub blink_every: Duration,
    pub breathe: bool,
    /// How long an expression holds before it settles back to neutral.
    pub settle_after: Duration,
    /// How long with no events before it sleeps. The next event wakes it.
    pub sleep_after: Duration,
}

impl Default for Idle {
    fn default() -> Self {
        Self {
            blink_every: Duration::from_secs(4),
            breathe: true,
            settle_after: Duration::from_secs(6),
            sleep_after: Duration::from_secs(300),
        }
    }
}

impl Rig {
    pub fn has(&self, expression: &str) -> bool {
        self.expressions.iter().any(|e| e == expression)
    }

    /// The file for one expression, or none when the rig cannot make it.
    pub fn state(&self, expression: &str) -> Option<PathBuf> {
        self.has(expression)
            .then(|| self.folder.join("states").join(format!("{expression}.svg")))
    }

    /// What `face.express` offers, which is what the rig contains and nothing
    /// else — minus `speaking`, which §11.2 says is driven by the speech port
    /// and never by a command.
    pub fn vocabulary(&self) -> Vec<String> {
        self.expressions
            .iter()
            .filter(|e| *e != "speaking")
            .cloned()
            .collect()
    }
}

/// Read one rig folder.
pub fn load(folder: &Path) -> Result<Rig, String> {
    let manifest = folder.join("rig.yaml");
    let text = std::fs::read_to_string(&manifest)
        .map_err(|e| format!("could not read {}: {e}", manifest.display()))?;
    let id = folder
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "rig".into());
    let mut rig = parse(&text, &id)?;
    rig.folder = folder.to_path_buf();

    // A manifest that promises an expression the folder does not have would
    // put it in the model's vocabulary and then fail when it was asked for.
    // Better to trust the folder: what is on disk is what the rig can do.
    let states = folder.join("states");
    rig.expressions
        .retain(|e| states.join(format!("{e}.svg")).exists());
    if rig.expressions.is_empty() {
        return Err(format!(
            "{} has a manifest but no states: expected files like states/neutral.svg",
            folder.display()
        ));
    }
    Ok(rig)
}

/// Every rig in a folder of rigs, in name order.
pub fn all(folder: &Path) -> Vec<Rig> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut found: Vec<Rig> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| load(&e.path()).ok())
        .collect();
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

/// A small reader for a small file.
///
/// `rig.yaml` is flat: scalars, one nested map, and two lists of plain strings.
/// A general YAML parser would be a dependency and a surface area for a format
/// that is deliberately this simple — and the `.loom` reader next door is
/// hand-written for the same reason.
fn parse(text: &str, id: &str) -> Result<Rig, String> {
    let mut rig = Rig {
        id: id.to_owned(),
        name: id.to_owned(),
        description: String::new(),
        expressions: Vec::new(),
        gestures: Vec::new(),
        gaze: false,
        idle: Idle::default(),
        folder: PathBuf::new(),
    };
    // Which list a `- item` belongs to, and whether we are inside `idle:`.
    let mut list: Option<&str> = None;
    let mut in_idle = false;

    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("");
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let indented = line.starts_with(' ');

        if let Some(item) = trimmed.strip_prefix("- ") {
            let item = unquote(item);
            match list {
                Some("expressions") => rig.expressions.push(item),
                Some("gestures") => rig.gestures.push(item),
                _ => {}
            }
            continue;
        }

        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let (key, value) = (key.trim(), unquote(value.trim()));
        if !indented {
            list = None;
            in_idle = false;
        }

        match (in_idle, key) {
            (true, "blinkEvery") => rig.idle.blink_every = every(&value, rig.idle.blink_every),
            (true, "breathe") => rig.idle.breathe = value != "false",
            (true, "settleAfter") => rig.idle.settle_after = every(&value, rig.idle.settle_after),
            (true, "sleepAfter") => rig.idle.sleep_after = every(&value, rig.idle.sleep_after),
            (_, "name") => rig.name = value,
            (_, "description") => rig.description = value,
            (_, "gaze") => rig.gaze = value == "true",
            (_, "idle") => in_idle = true,
            (_, "expressions" | "gestures") => {
                // `gestures: []` is an empty list said on one line.
                if value == "[]" {
                    list = None;
                } else {
                    list = Some(if key == "expressions" {
                        "expressions"
                    } else {
                        "gestures"
                    });
                }
            }
            _ => {}
        }
    }
    Ok(rig)
}

fn unquote(value: &str) -> String {
    value
        .trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .to_owned()
}

fn every(text: &str, fallback: Duration) -> Duration {
    super::memory::parse_window(text).unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rigs() -> PathBuf {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../rigs"))
    }

    #[test]
    fn the_four_reference_rigs_all_load() {
        let all = all(&rigs());
        let names: Vec<&str> = all.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["Line", "Orb", "Pixel", "Robot"]);
    }

    /// §11.2: every rig ships the same seven expressions and the gestures its
    /// form allows — except where the form cannot hold one.
    #[test]
    fn every_rig_has_the_expressions_its_form_allows() {
        for rig in all(&rigs()) {
            for must in [
                "neutral",
                "smile",
                "frown",
                "surprised",
                "thinking",
                "speaking",
                "love",
            ] {
                assert!(rig.has(must), "{} has no {must}", rig.name);
            }
        }
    }

    /// The vocabulary is generated from the rig, so a model driving a Pixel is
    /// never offered an expression a matrix cannot make.
    #[test]
    fn a_rig_that_cannot_sleep_does_not_offer_sleepy() {
        let pixel = load(&rigs().join("pixel")).unwrap();
        assert!(!pixel.has("sleepy"));
        assert!(!pixel.vocabulary().contains(&"sleepy".to_owned()));
        assert!(!pixel.gaze, "a matrix has nowhere to look");

        let line = load(&rigs().join("line")).unwrap();
        assert!(line.vocabulary().contains(&"sleepy".to_owned()));
        assert!(line.gaze);
    }

    /// §11.2: `speaking` is driven by the speech port, never by a command — so
    /// it is a state the rig has and not a word the model may say.
    #[test]
    fn speaking_is_a_state_and_not_a_command() {
        let line = load(&rigs().join("line")).unwrap();
        assert!(line.has("speaking"));
        assert!(!line.vocabulary().contains(&"speaking".to_owned()));
    }

    #[test]
    fn every_expression_in_the_vocabulary_has_a_file_to_draw() {
        for rig in all(&rigs()) {
            for expression in &rig.expressions {
                let file = rig.state(expression).expect("in the vocabulary");
                assert!(file.exists(), "{} has no {}", rig.name, file.display());
            }
        }
    }

    #[test]
    fn gestures_are_what_the_form_allows() {
        assert_eq!(
            load(&rigs().join("line")).unwrap().gestures,
            ["nod", "shake"]
        );
        assert!(load(&rigs().join("orb")).unwrap().gestures.is_empty());
    }

    #[test]
    fn idle_is_read_from_the_manifest() {
        let line = load(&rigs().join("line")).unwrap();
        assert_eq!(line.idle.blink_every, Duration::from_secs(4));
        assert_eq!(line.idle.settle_after, Duration::from_secs(6));
        assert_eq!(line.idle.sleep_after, Duration::from_secs(300));
        assert!(line.idle.breathe);
    }

    /// A manifest that promises what the folder does not have would put an
    /// expression in the model's vocabulary and then fail when asked for it.
    #[test]
    fn a_promise_the_folder_cannot_keep_is_dropped() {
        let dir = std::env::temp_dir().join(format!("cyberloom-rig-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("states")).unwrap();
        std::fs::write(dir.join("states/neutral.svg"), "<svg/>").unwrap();
        std::fs::write(
            dir.join("rig.yaml"),
            "name: Half\nexpressions:\n  - neutral\n  - moonwalk\n",
        )
        .unwrap();
        let rig = load(&dir).unwrap();
        assert_eq!(rig.expressions, ["neutral"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_with_a_manifest_and_no_states_says_so() {
        let dir = std::env::temp_dir().join(format!("cyberloom-rig-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("rig.yaml"),
            "name: Nothing\nexpressions:\n  - neutral\n",
        )
        .unwrap();
        let complaint = load(&dir).unwrap_err();
        assert!(complaint.contains("no states"), "{complaint}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
