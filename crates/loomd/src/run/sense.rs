//! Capturing from the world: a camera, a microphone, a screen, a speaker
//! (SPEC §6.4, §6.6).
//!
//! All of it through `ffmpeg`, which is a decision worth defending. The
//! alternative is binding V4L2 and PipeWire directly, which means unsafe
//! ioctls, a format-negotiation dance per device, and a different set of both
//! per platform — for the privilege of doing badly what one subprocess already
//! does well. ffmpeg speaks V4L2, ALSA, PipeWire and every pixel format a
//! webcam has ever produced, and it is on every machine this runs on.
//!
//! It also gives a machine with no camera a camera. `lavfi:` names one of
//! ffmpeg's synthetic sources, so a graph with a Webcam block runs on a laptop
//! with the lid shut, on a server, and in a test — which is the only reason
//! any of this could be written here at all.
//!
//! # Where frames go
//!
//! SPEC §12.3: frames and audio never leave the machine and are not recorded
//! unless the user turns recording on. So capture writes into a per-run
//! scratch folder that is deleted when the run ends, and `store` is what moves
//! it somewhere durable. The privacy default is not a setting the panel
//! promises and the engine ignores; it is where the file is written.

use super::value::Media;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Where a run's frames and audio live while it runs.
///
/// One folder per run, removed when it ends. A `Scratch` is the durable half
/// of the privacy default: nothing has to remember to clean up, because the
/// folder going away takes everything with it.
pub struct Scratch {
    dir: PathBuf,
    next: std::sync::atomic::AtomicU64,
}

/// So two scratch folders in one process can never be the same folder.
///
/// The pid and the run's name look unique enough and are not: two runs of the
/// same graph, a test suite running in parallel, a run restarted under the
/// same id — any of them gives two `Scratch`es one directory, and the first to
/// finish deletes the other's frames out from under it.
static NEXT_SCRATCH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl Scratch {
    pub fn open(run: &str) -> std::io::Result<Self> {
        let n = NEXT_SCRATCH.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("cyberloom-run-{}-{run}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            dir,
            next: std::sync::atomic::AtomicU64::new(0),
        })
    }

    pub fn path(&self) -> &Path {
        &self.dir
    }

    /// A name nothing else is using, inside this run's folder.
    pub fn file(&self, block: &str, extension: &str) -> PathBuf {
        let n = self.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.dir.join(format!("{block}-{n}.{extension}"))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // Frames and audio are not kept. Anything the user asked to keep was
        // copied out at the time (see `keep`).
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// How a device is named, and what ffmpeg should be told about it.
///
/// A path is a real device. `lavfi:` is one of ffmpeg's synthetic sources —
/// `lavfi:testsrc`, `lavfi:sine` — which is how a machine with no camera still
/// runs a graph that has a Webcam in it. A file is itself, which is what makes
/// a recorded clip a stand-in for a live device while a graph is being built.
#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Device(String),
    Synthetic(String),
    File(String),
}

impl Input {
    pub fn read(name: &str, kind: Kind) -> Self {
        let name = name.trim();
        if let Some(rest) = name.strip_prefix("lavfi:") {
            return Input::Synthetic(rest.to_owned());
        }
        if name.starts_with("/dev/") {
            return Input::Device(name.to_owned());
        }
        if Path::new(name).exists() {
            return Input::File(name.to_owned());
        }
        // Anything else is a device name the platform understands: `default`
        // for ALSA, a PipeWire node, a DirectShow name.
        let _ = kind;
        Input::Device(name.to_owned())
    }

    /// The `-f <format> -i <name>` pair ffmpeg needs.
    fn args(&self, kind: Kind) -> Vec<String> {
        match self {
            Input::Synthetic(source) => {
                vec!["-f".into(), "lavfi".into(), "-i".into(), source.clone()]
            }
            Input::File(path) => vec!["-i".into(), path.clone()],
            Input::Device(name) => match kind {
                Kind::Video => vec!["-f".into(), "v4l2".into(), "-i".into(), name.clone()],
                // ALSA reads a PipeWire device through its compatibility layer,
                // which is what every Linux desktop has had since 2021.
                Kind::Audio => vec!["-f".into(), "alsa".into(), "-i".into(), name.clone()],
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Video,
    Audio,
}

/// Take one frame.
pub fn frame(input: &Input, resolution: Option<&str>, into: &Path) -> Result<Media, String> {
    let mut args = input.args(Kind::Video);
    let mut after: Vec<String> = Vec::new();
    if let Some(size) = resolution.filter(|r| !r.trim().is_empty()) {
        let size = size.replace('×', "x");
        match input {
            // `-s` *before* `-i` asks the camera for that size, which is what
            // a camera should be asked: it captures at the size wanted rather
            // than capturing large and throwing pixels away.
            Input::Device(_) => {
                args.splice(0..0, ["-s".to_owned(), size]);
            }
            // A synthetic source and a file have no such negotiation — lavfi
            // rejects `-s` as an input option outright — so they are scaled
            // on the way out instead.
            _ => after.extend([
                "-vf".to_owned(),
                format!("scale={}", size.replace('x', ":")),
            ]),
        }
    }
    args.extend(after);
    args.extend(["-frames:v".into(), "1".into(), "-y".into()]);
    args.push(into.display().to_string());
    run_ffmpeg(&args, Some(input))?;
    describe(into, "image/png")
}

/// Record for a while.
pub fn audio(input: &Input, seconds: f64, into: &Path) -> Result<Media, String> {
    let mut args = input.args(Kind::Audio);
    args.extend([
        "-t".into(),
        format!("{seconds}"),
        // 16 kHz mono is what every speech model wants, and resampling once
        // here beats every downstream block doing it differently.
        "-ar".into(),
        "16000".into(),
        "-ac".into(),
        "1".into(),
        "-y".into(),
        into.display().to_string(),
    ]);
    run_ffmpeg(&args, Some(input))?;
    describe(into, "audio/wav")
}

/// Play a sound.
pub fn play(what: &Media, device: Option<&str>) -> Result<(), String> {
    let mut args = vec![
        "-i".to_owned(),
        what.path.clone(),
        "-f".into(),
        "alsa".into(),
    ];
    args.push(device.unwrap_or("default").to_owned());
    run_ffmpeg(&args, None).map(|_| ())
}

/// Copy a captured file somewhere durable, which is what recording means.
///
/// Only ever called when the user has turned recording on. The name carries
/// the time, because a folder of frames nobody can order is a folder of
/// frames nobody can use.
pub fn keep(what: &Media, into: &Path) -> Result<Media, String> {
    std::fs::create_dir_all(into).map_err(|e| format!("could not make {}: {e}", into.display()))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let extension = Path::new(&what.path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin");
    let kept = into.join(format!("{stamp}.{extension}"));
    std::fs::copy(&what.path, &kept)
        .map_err(|e| format!("could not record to {}: {e}", kept.display()))?;
    Ok(Media {
        path: kept.display().to_string(),
        mime: what.mime.clone(),
        bytes: what.bytes,
    })
}

fn run_ffmpeg(args: &[String], input: Option<&Input>) -> Result<String, String> {
    let output = Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        // Never wait for a person: ffmpeg asks before overwriting unless told.
        .arg("-nostdin")
        .args(args)
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                "ffmpeg is not installed, and the senses need it to reach a camera or a microphone"
                    .to_owned()
            }
            _ => format!("could not start ffmpeg: {e}"),
        })?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    // ffmpeg's last line is the one that says what went wrong; the rest is
    // build configuration nobody asked for.
    let why = String::from_utf8_lossy(&output.stderr)
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("ffmpeg failed with no message")
        .to_owned();
    Err(friendlier(&why, input))
}

/// Turn ffmpeg's message into one that says what to do.
///
/// Keyed off the input rather than off ffmpeg's wording. "Error opening input
/// files: No such file or directory" does not name the file, and matching on
/// the text would break the first time ffmpeg rephrased it — but the caller
/// knows perfectly well what it asked for.
fn friendlier(why: &str, input: Option<&Input>) -> String {
    match input {
        Some(Input::Device(name)) if why.contains("Permission denied") => {
            format!("{why} — `{name}` is there but this user cannot open it.")
        }
        Some(Input::Device(name)) if name.starts_with("/dev/video") => {
            format!("{why} — no camera at `{name}`. `lavfi:testsrc` stands in for one.")
        }
        Some(Input::Device(name)) => {
            format!("{why} — could not open `{name}`. `lavfi:sine` stands in for a microphone.")
        }
        Some(Input::File(path)) => format!("{why} — could not read `{path}`."),
        _ => why.to_owned(),
    }
}

fn describe(path: &Path, mime: &str) -> Result<Media, String> {
    let bytes = std::fs::metadata(path)
        .map_err(|e| format!("ffmpeg wrote nothing to {}: {e}", path.display()))?
        .len();
    Ok(Media {
        path: path.display().to_string(),
        mime: mime.to_owned(),
        bytes: bytes.min(u64::from(u32::MAX)) as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real frame, from a real ffmpeg. There is no camera on the machine
    /// running this, which is exactly the case `lavfi:` exists for.
    #[test]
    fn a_frame_is_captured_and_says_how_big_it_is() {
        let scratch = Scratch::open("frame-test").unwrap();
        let into = scratch.file("webcam", "png");
        let media = frame(
            &Input::Synthetic("testsrc=size=320x240:rate=1".into()),
            None,
            &into,
        )
        .unwrap();

        assert_eq!(media.mime, "image/png");
        assert!(
            media.bytes > 500,
            "a 320x240 frame is not {} bytes",
            media.bytes
        );
        assert!(Path::new(&media.path).exists());
    }

    /// Asking for a size asks the device for it, and the file that comes back
    /// really is that size.
    #[test]
    fn a_resolution_is_what_comes_back() {
        let scratch = Scratch::open("size-test").unwrap();
        let into = scratch.file("webcam", "png");
        frame(
            &Input::Synthetic("testsrc=rate=1".into()),
            Some("160x120"),
            &into,
        )
        .unwrap();

        let probe = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=width,height",
                "-of",
                "csv=p=0",
            ])
            .arg(&into)
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&probe.stdout).trim(), "160,120");
    }

    #[test]
    fn audio_is_recorded_at_the_rate_speech_models_want() {
        let scratch = Scratch::open("audio-test").unwrap();
        let into = scratch.file("mic", "wav");
        let media = audio(&Input::Synthetic("sine=frequency=440".into()), 0.25, &into).unwrap();
        assert_eq!(media.mime, "audio/wav");
        // A quarter second of 16 kHz mono 16-bit is about 8 kB.
        assert!(media.bytes > 4_000, "{} bytes", media.bytes);

        let probe = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_entries",
                "stream=sample_rate,channels",
                "-of",
                "csv=p=0",
            ])
            .arg(&into)
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&probe.stdout).trim(), "16000,1");
    }

    /// The privacy default is where the file is written, not a promise in a
    /// panel: the run's folder goes away and takes the frames with it
    /// (SPEC §12.3).
    #[test]
    fn a_runs_frames_go_away_with_the_run() {
        let kept;
        {
            let scratch = Scratch::open("privacy-test").unwrap();
            let into = scratch.file("webcam", "png");
            frame(&Input::Synthetic("testsrc=rate=1".into()), None, &into).unwrap();
            assert!(into.exists());
            kept = scratch.path().to_path_buf();
        }
        assert!(!kept.exists(), "the run's folder outlived the run");
    }

    /// Recording is what moves a frame somewhere durable.
    #[test]
    fn recording_copies_the_frame_out_of_the_scratch() {
        let durable = std::env::temp_dir().join(format!("cyberloom-kept-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&durable);

        let saved = {
            let scratch = Scratch::open("keep-test").unwrap();
            let into = scratch.file("webcam", "png");
            let media = frame(&Input::Synthetic("testsrc=rate=1".into()), None, &into).unwrap();
            keep(&media, &durable).unwrap()
        };

        assert!(
            Path::new(&saved.path).exists(),
            "the recording outlives the run"
        );
        assert!(saved.path.starts_with(durable.to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&durable);
    }

    /// A camera that is not there says so in words that name the fix.
    #[test]
    fn a_missing_camera_says_what_to_do_about_it() {
        let scratch = Scratch::open("missing-test").unwrap();
        let into = scratch.file("webcam", "png");
        let why = frame(&Input::Device("/dev/video-nope".into()), None, &into).unwrap_err();
        assert!(why.contains("lavfi:testsrc"), "{why}");
    }

    #[test]
    fn a_device_name_is_read_for_what_it_is() {
        assert_eq!(
            Input::read("/dev/video0", Kind::Video),
            Input::Device("/dev/video0".into())
        );
        assert_eq!(
            Input::read("lavfi:testsrc", Kind::Video),
            Input::Synthetic("testsrc".into())
        );
        assert_eq!(
            Input::read("default", Kind::Audio),
            Input::Device("default".into())
        );
        // An existing file stands in for a device while a graph is built.
        let clip = std::env::temp_dir().join(format!("cyberloom-clip-{}.png", std::process::id()));
        std::fs::write(&clip, b"x").unwrap();
        assert_eq!(
            Input::read(clip.to_str().unwrap(), Kind::Video),
            Input::File(clip.display().to_string())
        );
        let _ = std::fs::remove_file(&clip);
    }

    /// Two captures in one run never collide, however fast they come.
    #[test]
    fn frames_do_not_overwrite_each_other() {
        let scratch = Scratch::open("names-test").unwrap();
        let names: Vec<PathBuf> = (0..50).map(|_| scratch.file("webcam", "png")).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(unique.len(), names.len());
    }
}
