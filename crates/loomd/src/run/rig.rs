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
///
/// The rig's manifest sets the defaults, because a rig knows how it should
/// move; the block's own settings override them, because a graph knows how
/// long its assistant should hold a face. `Idle::for_block` applies the second
/// to the first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Idle {
    pub blink_every: Duration,
    /// Breaths a minute; zero is a face that does not breathe.
    pub breathe_per_min: u32,
    /// How long an expression holds before it settles back to neutral.
    pub settle_after: Duration,
    /// How long with no events before it sleeps. The next event wakes it.
    pub sleep_after: Duration,
}

/// A resting breath. Thirteen a minute is a person at ease; it is also the
/// 4.6 s cycle the shell used before the number was anyone's to choose.
pub const BREATHE_PER_MIN: u32 = 13;

/// How long `surprised` holds: "a beat, then settles" (SPEC §11.2).
pub const BEAT: Duration = Duration::from_millis(1500);

impl Default for Idle {
    fn default() -> Self {
        Self {
            blink_every: Duration::from_secs(4),
            breathe_per_min: BREATHE_PER_MIN,
            settle_after: Duration::from_secs(6),
            sleep_after: Duration::from_secs(300),
        }
    }
}

impl Idle {
    /// The rig's idle, overridden by whatever the block's settings say.
    ///
    /// Each setting is read only when it is set: an inspector field left blank
    /// means "what the rig says", not zero.
    pub fn for_block(self, block: &graph_format::Block) -> Idle {
        use super::blocks::{number, setting};
        let mut idle = self;
        if let Some(text) = setting(block, "blink")
            && let Some(every) = super::memory::parse_window(text)
        {
            idle.blink_every = every;
        }
        if let Some(per_min) = number(block, "breathePerMin") {
            idle.breathe_per_min = per_min.max(0.0).round() as u32;
        }
        if let Some(seconds) = number(block, "settleSec") {
            idle.settle_after = Duration::from_secs_f64(seconds.max(0.0));
        }
        if let Some(minutes) = number(block, "sleepAfterMin") {
            idle.sleep_after = Duration::from_secs_f64(minutes.max(0.0) * 60.0);
        }
        idle
    }

    /// How long this expression holds before idle takes it back to neutral.
    ///
    /// A settle timer of zero means "do not": a face that should hold what it
    /// was told until told otherwise, which is what a graph that drives every
    /// change itself wants.
    pub fn holds(&self, expression: &str) -> Duration {
        match expression {
            "neutral" | "sleepy" => Duration::MAX,
            _ if self.settle_after.is_zero() => Duration::MAX,
            "surprised" => BEAT.min(self.settle_after),
            _ => self.settle_after,
        }
    }
}

/// The colour a mood is, for the media that have one and not a face: a Status
/// light breathes it, a block on the canvas can glow it (SPEC §11.7). The same
/// vocabulary, so the same table; it lives here rather than in the shell so
/// that a lamp on a serial port and a swatch in the window agree.
pub fn colour_for(expression: &str) -> &'static str {
    match expression {
        "smile" => "#6fc98a",
        "frown" => "#e0a04f",
        "surprised" => "#e8ebf0",
        "thinking" => "#8f7be8",
        "love" => "#e0685f",
        "sleepy" => "#3a4250",
        // neutral, speaking, and anything a user's rig added.
        _ => "#56c7d6",
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

    /// The drawings, by expression, for a shell that does not have them.
    ///
    /// The four reference rigs are bundled into the shell; a rig the user put in
    /// their workspace is not, and this is how its faces reach the window. A
    /// state that carries script is left out: rigs are content, not code
    /// (SPEC §11.1), and a drawing that wanted to run something is not a
    /// drawing.
    pub fn states(&self) -> std::collections::BTreeMap<String, String> {
        self.expressions
            .iter()
            .filter_map(|e| {
                let text = std::fs::read_to_string(self.state(e)?).ok()?;
                (!looks_like_code(&text)).then(|| (e.clone(), text))
            })
            .collect()
    }
}

/// Whether an SVG is trying to be a program.
///
/// Case-insensitive on purpose, and matching on the shape of a handler
/// attribute rather than a list of names: the point is not to catch every
/// trick, it is that a rig's author never had a reason to write any of these.
pub fn looks_like_code(svg: &str) -> bool {
    let lower = svg.to_ascii_lowercase();
    lower.contains("<script")
        || lower.contains("javascript:")
        || lower.contains("<foreignobject")
        || lower
            .split(|c: char| c.is_whitespace() || c == '<')
            .any(|token| token.starts_with("on") && token.contains('='))
}

/// An 8 × 8 matrix rig's state as the bits a physical matrix would show.
///
/// This is what `face.render` sends a USB device block (SPEC §11.5). The
/// drawing is read rather than a second copy of it kept: each cell is a square
/// `<rect>`, the darkest fill among the cells is "off", and everything else is
/// lit. Rows are bytes, top first; the high bit is the left column. A rig that
/// is not a grid of sixty-four squares gets `None`, and its face goes to the
/// device as words instead.
pub fn matrix_bits(svg: &str) -> Option<[u8; 8]> {
    struct Cell {
        x: f64,
        y: f64,
        fill: String,
    }
    let mut cells: Vec<Cell> = Vec::new();
    for tag in svg.split("<rect").skip(1) {
        let tag = tag.split('>').next().unwrap_or("");
        let attr = |name: &str| -> Option<String> {
            let at = tag.find(&format!(" {name}=\""))?;
            let rest = &tag[at + name.len() + 3..];
            Some(rest.split('"').next()?.to_owned())
        };
        let (Some(w), Some(h)) = (attr("width"), attr("height")) else {
            continue;
        };
        if w != h {
            continue;
        }
        let (Some(x), Some(y), Some(fill)) = (attr("x"), attr("y"), attr("fill")) else {
            continue;
        };
        if let (Ok(x), Ok(y)) = (x.parse::<f64>(), y.parse::<f64>()) {
            cells.push(Cell { x, y, fill });
        }
    }
    if cells.len() != 64 {
        return None;
    }
    let mut xs: Vec<f64> = cells.iter().map(|c| c.x).collect();
    let mut ys: Vec<f64> = cells.iter().map(|c| c.y).collect();
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    ys.sort_by(f64::total_cmp);
    ys.dedup();
    if xs.len() != 8 || ys.len() != 8 {
        return None;
    }
    let off = cells
        .iter()
        .map(|c| c.fill.as_str())
        .min_by_key(|fill| luminance(fill))?
        .to_owned();
    let mut rows = [0u8; 8];
    for cell in &cells {
        if cell.fill == off {
            continue;
        }
        let col = xs.iter().position(|x| *x == cell.x)?;
        let row = ys.iter().position(|y| *y == cell.y)?;
        rows[row] |= 0x80 >> col;
    }
    Some(rows)
}

/// Roughly how bright a `#rrggbb` is. Anything that is not one counts as
/// bright, so a named colour is never mistaken for "off".
fn luminance(fill: &str) -> u32 {
    let hex = fill.strip_prefix('#').unwrap_or("");
    if hex.len() != 6 {
        return u32::MAX;
    }
    let channel = |at: usize| u32::from_str_radix(&hex[at..at + 2], 16).unwrap_or(0);
    channel(0) * 299 + channel(2) * 587 + channel(4) * 114
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
            (true, "breathe") => {
                if value == "false" {
                    rig.idle.breathe_per_min = 0;
                }
            }
            (true, "breathePerMin") => {
                rig.idle.breathe_per_min = value.parse().unwrap_or(rig.idle.breathe_per_min)
            }
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
        assert_eq!(line.idle.breathe_per_min, BREATHE_PER_MIN);
    }

    /// The block's settings win over the manifest, and only where they are set.
    #[test]
    fn a_blocks_settings_override_the_rigs_idle_where_they_are_set() {
        let line = load(&rigs().join("line")).unwrap();
        let mut block = block("face", "avatar");
        block
            .settings
            .insert("settleSec".into(), graph_format::Setting::Float(1.5));
        block
            .settings
            .insert("breathePerMin".into(), graph_format::Setting::Int(0));
        let idle = line.idle.for_block(&block);
        assert_eq!(idle.settle_after, Duration::from_millis(1500));
        assert_eq!(idle.breathe_per_min, 0);
        // Not set, so the rig's own.
        assert_eq!(idle.blink_every, Duration::from_secs(4));
        assert_eq!(idle.sleep_after, Duration::from_secs(300));
    }

    /// "surprised: a beat, then settles"; "neutral: idle returns here".
    #[test]
    fn a_surprise_is_a_beat_and_neutral_holds_forever() {
        let idle = Idle::default();
        assert_eq!(idle.holds("surprised"), BEAT);
        assert_eq!(idle.holds("smile"), Duration::from_secs(6));
        assert_eq!(idle.holds("neutral"), Duration::MAX);
        assert_eq!(idle.holds("sleepy"), Duration::MAX);
        let never = Idle {
            settle_after: Duration::ZERO,
            ..Idle::default()
        };
        assert_eq!(never.holds("smile"), Duration::MAX);
    }

    fn block(id: &str, kind: &str) -> graph_format::Block {
        graph_format::Block {
            id: id.into(),
            kind: kind.into(),
            title: None,
            position: graph_format::Position { x: 0.0, y: 0.0 },
            size: None,
            view: graph_format::View::Summary,
            settings: Default::default(),
            ports: Vec::new(),
            source: None,
            disabled: false,
            breakpoint: false,
            frame: None,
        }
    }

    /// The states reach a shell that lacks them, and a state that is a program
    /// does not.
    #[test]
    fn states_are_handed_over_and_a_script_is_not() {
        let line = load(&rigs().join("line")).unwrap();
        let states = line.states();
        assert_eq!(states.len(), line.expressions.len());
        assert!(states["neutral"].contains("<svg"));

        assert!(looks_like_code("<svg><script>alert(1)</script></svg>"));
        assert!(looks_like_code("<svg onload=\"x()\"></svg>"));
        assert!(looks_like_code("<svg><a href=\"javascript:x\"/></svg>"));
        assert!(!looks_like_code(&states["neutral"]));
        // "one" and "only" are words, not handlers.
        assert!(!looks_like_code("<svg><title>only one</title></svg>"));
    }

    /// The Pixel rig's drawings are the bits a matrix would show (SPEC §11.5).
    #[test]
    fn a_pixel_state_becomes_matrix_bits() {
        let pixel = load(&rigs().join("pixel")).unwrap();
        let read = |e: &str| {
            let svg = std::fs::read_to_string(pixel.state(e).unwrap()).unwrap();
            matrix_bits(&svg).unwrap_or_else(|| panic!("{e} is not a matrix"))
        };
        let love = read("love");
        // A heart: two bumps on the top row, a full row under them, a point.
        assert_eq!(love[0], 0b0110_0110, "{love:?}");
        assert_eq!(love[1], 0b1111_1100, "{love:?}");
        assert!(love[7] == 0, "{love:?}");
        let neutral = read("neutral");
        assert_ne!(neutral, love);
        assert!(neutral.iter().any(|row| *row != 0));

        // A rig that is not a grid gets words instead.
        let line = load(&rigs().join("line")).unwrap();
        let svg = std::fs::read_to_string(line.state("neutral").unwrap()).unwrap();
        assert!(matrix_bits(&svg).is_none());
    }

    #[test]
    fn every_mood_has_a_colour_and_unknown_moods_share_neutrals() {
        assert_eq!(colour_for("love"), "#e0685f");
        assert_eq!(colour_for("neutral"), colour_for("moonwalk"));
        assert_ne!(colour_for("smile"), colour_for("frown"));
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

    /// The layouts a packaged build actually uses.
    ///
    /// This is here rather than in `runner` because it is about rigs, and
    /// because the bug it pins down — an AppImage whose avatar had no face —
    /// was found by building one and looking, not by reading the code.
    #[test]
    fn rigs_are_found_where_a_package_puts_them() {
        let dir = std::env::temp_dir().join(format!("cyberloom-layout-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (binary, resources) in [
            ("usr/bin/cyberloom", "usr/bin/rigs"),
            ("usr/bin/cyberloom", "usr/lib/Cyberloom/rigs"),
            ("usr/bin/cyberloom", "usr/share/cyberloom/rigs"),
            ("opt/cyberloom/cyberloom", "opt/cyberloom/rigs"),
            // What scripts/install.sh lays down: a launcher on the PATH, and
            // the binary and rigs together under lib/.
            (
                "usr/local/lib/cyberloom/cyberloom",
                "usr/local/lib/cyberloom/rigs",
            ),
        ] {
            let _ = std::fs::remove_dir_all(&dir);
            let exe = dir.join(binary);
            std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
            std::fs::write(&exe, b"").unwrap();
            let rigs = dir.join(resources).join("line");
            std::fs::create_dir_all(&rigs).unwrap();
            std::fs::write(rigs.join("rig.yaml"), "name: Line\n").unwrap();
            let found = crate::run::runner::rigs_near(Some(&exe))
                .unwrap_or_else(|| panic!("nothing found for {resources}"));
            assert!(
                found.join("line/rig.yaml").is_file(),
                "{resources}: found {}",
                found.display()
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_binary_with_no_rigs_anywhere_near_it_finds_none() {
        let dir = std::env::temp_dir().join(format!("cyberloom-bare-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("cyberloom");
        std::fs::write(&exe, b"").unwrap();
        assert!(crate::run::runner::rigs_near(Some(&exe)).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
