//! Reading what a frame or a sound contains (SPEC §6.1).
//!
//! Object detection, face recognition, speech, affect. A trait with two
//! implementations, for the same reason the model provider has two: one that
//! runs real models, and one that answers from a script so the rest of the
//! engine can be tested without weights on disk.
//!
//! # Why the real one goes through Python
//!
//! SPEC §13.4 already puts face recognition "via the Python runtime", and the
//! rest belongs beside it. Every one of these models ships as ONNX or as a
//! Python package, the ecosystem's tooling for loading them is Python, and
//! Cyberloom already runs Python for custom blocks — so a perception model is
//! a custom block the engine wrote, running the same way, in the same
//! environment the user can inspect and replace.
//!
//! The alternative is linking an inference runtime into the daemon, which
//! would make the engine's binary depend on the accelerator a machine happens
//! to have, and make swapping a model a rebuild.
//!
//! # What is proven, and what is not
//!
//! The scripted provider pins down every path through the engine: what is
//! asked for, what comes back, how a detection becomes a value, what happens
//! when a model is missing. What it cannot tell you is whether yolo-v8n finds
//! a door — that is a question about the model, and it wants weights and a
//! machine that can run them.

use super::value::{Media, Value};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One thing seen in a frame.
///
/// The numbers are `f64` although every detector produces `f32`. They cross
/// JSON, which has one number type and it is a double, and they are shown to
/// people — `0.8799999952316284` is what an `f32` confidence looks like by the
/// time it reaches a panel. The width is chosen for the boundary rather than
/// for the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Seen {
    pub label: String,
    pub confidence: f64,
    /// `[x, y, w, h]` in pixels, from the top left.
    ///
    /// `box` is a Rust keyword, which is this field's problem and nobody
    /// else's: it is `box` on the wire.
    #[serde(rename = "box")]
    pub box_: [f64; 4],
}

/// Valence and arousal, read from a stream of text (SPEC §6.1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Affect {
    /// −1 miserable, +1 delighted.
    pub valence: f64,
    /// 0 calm, 1 agitated.
    pub arousal: f64,
}

impl Affect {
    /// The expression an Avatar should wear (SPEC §11.2).
    ///
    /// This is the whole point of the Affect block: a smile that costs no tool
    /// call. The thresholds are deliberately wide — an avatar that changed
    /// face on every comma would be worse than one that did not move.
    pub fn expression(self) -> &'static str {
        match (self.valence, self.arousal) {
            (v, a) if v > 0.35 && a > 0.6 => "love",
            (v, _) if v > 0.25 => "smile",
            (v, _) if v < -0.25 => "frown",
            (_, a) if a > 0.7 => "surprised",
            _ => "neutral",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PerceptionError {
    /// The model is not on this machine. Its own case, because it is the one
    /// a person can do something about (SPEC §15.13).
    NoModel {
        what: String,
        hint: String,
    },
    Failed(String),
}

impl std::fmt::Display for PerceptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PerceptionError::NoModel { what, hint } => {
                write!(f, "no model for {what}: {hint}")
            }
            PerceptionError::Failed(why) => f.write_str(why),
        }
    }
}

pub trait Perception: Send + Sync {
    fn name(&self) -> &str;

    /// What is in this frame.
    fn detect(&self, image: &Media, model: &str) -> Result<Vec<Seen>, PerceptionError>;

    /// Who is in this frame.
    ///
    /// Returns an embedding and, when it matches someone enrolled, a name.
    /// Never an image: SPEC §12.3 is explicit that faces are stored as
    /// embeddings, and a signature that could return a crop would make that a
    /// promise rather than a property.
    fn recognise(&self, image: &Media, threshold: f64) -> Result<Person, PerceptionError>;

    fn transcribe(&self, audio: &Media, model: &str) -> Result<Heard, PerceptionError>;

    fn speak(&self, text: &str, voice: &str, into: &Path) -> Result<Media, PerceptionError>;

    /// A label with a confidence (SPEC §6.1).
    fn classify(&self, text: &str, labels: &[String]) -> Result<(String, f64), PerceptionError>;

    fn affect(&self, text: &str) -> Result<Affect, PerceptionError>;

    fn embed(&self, text: &str, model: &str) -> Result<Vec<f32>, PerceptionError>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Person {
    /// The name of the enrolled person, when one matched.
    pub name: Option<String>,
    pub confidence: f64,
    /// How many numbers the embedding has. The embedding itself does not
    /// leave the recogniser: nothing downstream of a face has any use for 512
    /// floats, and putting them on a wire would put them in a graph file.
    pub dimensions: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Heard {
    pub text: String,
    /// How long the audio was, so a panel can show the lag (SPEC §6.1).
    pub seconds: f64,
}

// --------------------------------------------------------------- the real one

/// Perception through the Python runtime.
pub struct Local {
    /// The folder models are looked for in, and downloaded into on first run
    /// (SPEC §15.13).
    pub models: PathBuf,
    /// The interpreter, which is the workspace's if it has one.
    pub python: PathBuf,
}

impl Local {
    pub fn new(models: PathBuf, python: PathBuf) -> Self {
        Self { models, python }
    }

    /// Run one of the engine's own Python helpers and read its answer.
    ///
    /// The same shape as a custom block: a driver on disk, arguments as JSON,
    /// the result in a file. A perception model is a custom block the engine
    /// wrote, which is why it runs the way one does.
    fn ask(
        &self,
        task: &str,
        request: serde_json::Value,
    ) -> Result<serde_json::Value, PerceptionError> {
        let driver = self.models.join("perceive.py");
        if !driver.exists() {
            return Err(PerceptionError::NoModel {
                what: task.to_owned(),
                hint: format!(
                    "the perception helper is not in {}. It is fetched on first run, \
                     which needs the network once (SPEC §15.13).",
                    self.models.display()
                ),
            });
        }

        let out = std::env::temp_dir().join(format!(
            "cyberloom-perceive-{}-{task}.json",
            std::process::id()
        ));
        let result = std::process::Command::new(&self.python)
            .arg(&driver)
            .arg(task)
            .arg(request.to_string())
            .arg(&out)
            .output()
            .map_err(|e| PerceptionError::Failed(format!("could not start python: {e}")))?;

        if !result.status.success() {
            let why = String::from_utf8_lossy(&result.stderr)
                .lines()
                .rev()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .unwrap_or("the helper failed with no message")
                .to_owned();
            // A missing model is a different thing from a broken one, and the
            // difference is what the panel offers to do about it.
            if why.contains("ModuleNotFoundError") || why.contains("No such file") {
                return Err(PerceptionError::NoModel {
                    what: task.to_owned(),
                    hint: why,
                });
            }
            return Err(PerceptionError::Failed(why));
        }

        let text = std::fs::read_to_string(&out)
            .map_err(|e| PerceptionError::Failed(format!("the helper wrote nothing: {e}")))?;
        let _ = std::fs::remove_file(&out);
        serde_json::from_str(&text)
            .map_err(|e| PerceptionError::Failed(format!("the helper's answer is not JSON: {e}")))
    }
}

impl Perception for Local {
    fn name(&self) -> &str {
        "local"
    }

    fn detect(&self, image: &Media, model: &str) -> Result<Vec<Seen>, PerceptionError> {
        let answer = self.ask(
            "detect",
            serde_json::json!({ "image": image.path, "model": model }),
        )?;
        serde_json::from_value(answer)
            .map_err(|e| PerceptionError::Failed(format!("detections came back wrong: {e}")))
    }

    fn recognise(&self, image: &Media, threshold: f64) -> Result<Person, PerceptionError> {
        let answer = self.ask(
            "recognise",
            serde_json::json!({ "image": image.path, "threshold": threshold }),
        )?;
        serde_json::from_value(answer)
            .map_err(|e| PerceptionError::Failed(format!("the face came back wrong: {e}")))
    }

    fn transcribe(&self, audio: &Media, model: &str) -> Result<Heard, PerceptionError> {
        let answer = self.ask(
            "transcribe",
            serde_json::json!({ "audio": audio.path, "model": model }),
        )?;
        serde_json::from_value(answer)
            .map_err(|e| PerceptionError::Failed(format!("the transcript came back wrong: {e}")))
    }

    fn speak(&self, text: &str, voice: &str, into: &Path) -> Result<Media, PerceptionError> {
        self.ask(
            "speak",
            serde_json::json!({ "text": text, "voice": voice, "into": into.display().to_string() }),
        )?;
        let bytes = std::fs::metadata(into)
            .map_err(|e| PerceptionError::Failed(format!("no audio was written: {e}")))?
            .len();
        Ok(Media {
            path: into.display().to_string(),
            mime: "audio/wav".into(),
            bytes: bytes.min(u64::from(u32::MAX)) as u32,
            said: Some(text.to_owned()),
        })
    }

    fn classify(&self, text: &str, labels: &[String]) -> Result<(String, f64), PerceptionError> {
        let answer = self.ask(
            "classify",
            serde_json::json!({ "text": text, "labels": labels }),
        )?;
        let label = answer
            .get("label")
            .and_then(|l| l.as_str())
            .ok_or_else(|| PerceptionError::Failed("no label came back".into()))?;
        let confidence = answer
            .get("confidence")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        Ok((label.to_owned(), confidence))
    }

    fn affect(&self, text: &str) -> Result<Affect, PerceptionError> {
        let answer = self.ask("affect", serde_json::json!({ "text": text }))?;
        serde_json::from_value(answer)
            .map_err(|e| PerceptionError::Failed(format!("affect came back wrong: {e}")))
    }

    fn embed(&self, text: &str, model: &str) -> Result<Vec<f32>, PerceptionError> {
        let answer = self.ask("embed", serde_json::json!({ "text": text, "model": model }))?;
        serde_json::from_value(answer)
            .map_err(|e| PerceptionError::Failed(format!("the embedding came back wrong: {e}")))
    }
}

// ----------------------------------------------------------- the scripted one

/// Perception from a script.
///
/// Everything it returns was written down in advance. What it exists for is
/// the same thing the scripted model provider exists for: a test that needed
/// weights on disk and a machine that can run them is not a test of the
/// engine, it is a test of somebody else's model.
#[derive(Default)]
pub struct Scripted {
    pub detections: Vec<Seen>,
    pub person: Option<Person>,
    pub transcript: Option<Heard>,
    pub label: Option<(String, f64)>,
    pub mood: Option<Affect>,
    /// Every request it was given, so a test can assert what was asked.
    pub seen: std::sync::Mutex<Vec<String>>,
}

impl Scripted {
    pub fn sees(labels: &[(&str, f64)]) -> Self {
        Self {
            detections: labels
                .iter()
                .map(|(label, confidence)| Seen {
                    label: (*label).to_owned(),
                    confidence: *confidence,
                    box_: [0.0, 0.0, 10.0, 10.0],
                })
                .collect(),
            ..Default::default()
        }
    }
}

impl Perception for Scripted {
    fn name(&self) -> &str {
        "scripted"
    }

    fn detect(&self, image: &Media, _model: &str) -> Result<Vec<Seen>, PerceptionError> {
        self.seen
            .lock()
            .unwrap()
            .push(format!("detect {}", image.path));
        Ok(self.detections.clone())
    }

    fn recognise(&self, image: &Media, _threshold: f64) -> Result<Person, PerceptionError> {
        self.seen
            .lock()
            .unwrap()
            .push(format!("recognise {}", image.path));
        self.person.clone().ok_or_else(|| PerceptionError::NoModel {
            what: "face recognition".into(),
            hint: "the script has no face".into(),
        })
    }

    fn transcribe(&self, audio: &Media, _model: &str) -> Result<Heard, PerceptionError> {
        self.seen
            .lock()
            .unwrap()
            .push(format!("transcribe {}", audio.path));
        self.transcript
            .clone()
            .ok_or_else(|| PerceptionError::NoModel {
                what: "speech to text".into(),
                hint: "the script has no transcript".into(),
            })
    }

    fn speak(&self, text: &str, _voice: &str, into: &Path) -> Result<Media, PerceptionError> {
        self.seen.lock().unwrap().push(format!("speak {text}"));
        // A sound of about the right length, so a graph downstream of a voice
        // has real audio to work on: a soft tone in syllable-sized bursts,
        // which is enough for a mouth to move to. Written here rather than by
        // ffmpeg, so a scripted voice works on a machine without one.
        let rate = 16_000u32;
        let seconds = (text.split_whitespace().count() as f64 / 2.5).max(0.2);
        let samples: Vec<i16> = (0..(seconds * f64::from(rate)) as u32)
            .map(|i| {
                let t = f64::from(i) / f64::from(rate);
                let burst = (t * 1000.0) as u64 % 300 < 180;
                if burst {
                    ((t * 220.0 * std::f64::consts::TAU).sin() * 0.25 * f64::from(i16::MAX)) as i16
                } else {
                    0
                }
            })
            .collect();
        let bytes = super::sense::wav(&samples, rate);
        std::fs::write(into, &bytes).map_err(|e| {
            PerceptionError::Failed(format!("could not write {}: {e}", into.display()))
        })?;
        Ok(Media {
            path: into.display().to_string(),
            mime: "audio/wav".into(),
            bytes: bytes.len() as u32,
            said: Some(text.to_owned()),
        })
    }

    fn classify(&self, text: &str, _labels: &[String]) -> Result<(String, f64), PerceptionError> {
        self.seen.lock().unwrap().push(format!("classify {text}"));
        self.label
            .clone()
            .ok_or_else(|| PerceptionError::Failed("the script has no label".into()))
    }

    fn affect(&self, text: &str) -> Result<Affect, PerceptionError> {
        self.seen.lock().unwrap().push(format!("affect {text}"));
        self.mood
            .ok_or_else(|| PerceptionError::Failed("the script has no mood".into()))
    }

    fn embed(&self, text: &str, _model: &str) -> Result<Vec<f32>, PerceptionError> {
        self.seen.lock().unwrap().push(format!("embed {text}"));
        // A deterministic vector: the same text always embeds the same way,
        // which is what makes a similarity test mean anything.
        Ok((0..8)
            .map(|i| ((text.len() + i) as f32 * 0.125).sin())
            .collect())
    }
}

/// Detections as the value the `objects` port carries (SPEC §6.1).
pub fn detections_as_value(seen: &[Seen]) -> Value {
    Value::Data(serde_json::json!({
        "count": seen.len(),
        "labels": seen.iter().map(|s| s.label.clone()).collect::<Vec<_>>(),
        "objects": seen,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> Media {
        Media {
            path: "/tmp/frame.png".into(),
            mime: "image/png".into(),
            bytes: 1000,
            said: None,
        }
    }

    #[test]
    fn detections_carry_labels_confidences_and_boxes() {
        let eye = Scripted::sees(&[("door", 0.88), ("person", 0.42)]);
        let seen = eye.detect(&frame(), "yolo-v8n").unwrap();
        let value = detections_as_value(&seen);

        let Value::Data(json) = value else {
            panic!("detections are a record")
        };
        assert_eq!(json["count"], 2);
        assert_eq!(json["labels"], serde_json::json!(["door", "person"]));
        assert_eq!(json["objects"][0]["confidence"], 0.88);
        // `box` on the wire, whatever Rust has to call the field.
        assert_eq!(json["objects"][0]["box"][2], 10.0);
    }

    /// SPEC §6.1: affect feeds an Avatar's `express` port, so a smile costs no
    /// tool call. The mapping from valence and arousal to a face is the whole
    /// of that promise.
    #[test]
    fn affect_becomes_a_face() {
        let face = |valence, arousal| Affect { valence, arousal }.expression();
        assert_eq!(face(0.8, 0.8), "love");
        assert_eq!(face(0.6, 0.2), "smile");
        assert_eq!(face(-0.6, 0.3), "frown");
        assert_eq!(face(0.0, 0.9), "surprised");
        assert_eq!(face(0.0, 0.1), "neutral");
    }

    /// The thresholds are wide on purpose: an avatar that changed face on
    /// every comma would be worse than one that did not move.
    #[test]
    fn a_small_change_in_mood_does_not_change_the_face() {
        for valence in [-0.2, -0.1, 0.0, 0.1, 0.2] {
            assert_eq!(
                Affect {
                    valence,
                    arousal: 0.3
                }
                .expression(),
                "neutral",
                "valence {valence} should still be neutral"
            );
        }
    }

    /// A face never leaves as an image (SPEC §12.3). The type says so: there
    /// is nowhere in `Person` to put one.
    #[test]
    fn a_recognised_face_is_an_embedding_and_a_name() {
        let eye = Scripted {
            person: Some(Person {
                name: Some("mykl".into()),
                confidence: 0.91,
                dimensions: 512,
            }),
            ..Default::default()
        };
        let who = eye.recognise(&frame(), 0.6).unwrap();
        assert_eq!(who.name.as_deref(), Some("mykl"));
        assert_eq!(who.dimensions, 512);

        // And what it serialises to has no image in it either — this is the
        // check that would fail if a crop were ever added for convenience.
        let json = serde_json::to_value(&who).unwrap();
        let keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys, ["confidence", "dimensions", "name"]);
    }

    /// A missing model is its own kind of failure, because it is the one a
    /// person can do something about (SPEC §15.13).
    #[test]
    fn a_missing_model_says_so_rather_than_failing_vaguely() {
        let nothing = Scripted::default();
        let err = nothing.recognise(&frame(), 0.6).unwrap_err();
        assert!(matches!(err, PerceptionError::NoModel { .. }), "{err:?}");
        assert!(err.to_string().contains("face recognition"), "{err}");
    }

    /// The local provider looks for its helper before it runs anything, so a
    /// machine without the models says which folder is empty rather than
    /// producing a Python traceback.
    #[test]
    fn the_local_provider_names_the_folder_it_looked_in() {
        let local = Local::new(
            PathBuf::from("/tmp/definitely-not-here"),
            PathBuf::from("python3"),
        );
        let err = local.detect(&frame(), "yolo-v8n").unwrap_err();
        let PerceptionError::NoModel { hint, .. } = err else {
            panic!("expected a missing model");
        };
        assert!(hint.contains("/tmp/definitely-not-here"), "{hint}");
        assert!(hint.contains("first run"), "{hint}");
    }

    /// The same text always embeds the same way, which is what makes a
    /// similarity comparison mean anything.
    #[test]
    fn embedding_is_deterministic() {
        let eye = Scripted::default();
        assert_eq!(
            eye.embed("hello", "any").unwrap(),
            eye.embed("hello", "any").unwrap()
        );
        assert_ne!(
            eye.embed("hello", "any").unwrap(),
            eye.embed("a longer sentence", "any").unwrap()
        );
    }

    /// A scripted voice produces real audio, so a graph downstream of it has
    /// something to work on rather than a path to nothing.
    #[test]
    fn a_scripted_voice_still_makes_a_sound() {
        let into = std::env::temp_dir().join(format!("cyberloom-say-{}.wav", std::process::id()));
        let eye = Scripted::default();
        let said = eye.speak("four words go here", "any", &into).unwrap();
        assert_eq!(said.mime, "audio/wav");
        assert!(said.bytes > 1000, "{} bytes", said.bytes);
        // The audio remembers its words, which is what auto-affect reads.
        assert_eq!(said.said.as_deref(), Some("four words go here"));
        let _ = std::fs::remove_file(&into);
    }
}
