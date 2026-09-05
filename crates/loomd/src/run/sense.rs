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
        said: None,
    })
}

/// How wide a preview is.
///
/// Small enough that a frame every 15th of a second is a few kilobytes rather
/// than a megabyte, and large enough to see what the camera is pointed at.
/// This is a *thumbnail* and the distinction is load-bearing: a full frame
/// crossing the socket as base64 would make the protocol the bottleneck, which
/// is exactly why a captured frame travels as a path (`run::value::Media`).
pub const PREVIEW_WIDTH: u32 = 160;

/// A small copy of a frame, as a data URI the shell can put in an `<img>`.
///
/// Figure 6's Live section shows what the camera sees, and there is no way to
/// show it without the pixels reaching the window: a path is meaningless to a
/// browser and a Tauri asset protocol would let the shell read any file the
/// engine can. A thumbnail is the smallest thing that answers the question.
pub fn preview(what: &Media) -> Result<String, String> {
    let small = std::env::temp_dir().join(format!(
        "cyberloom-preview-{}-{}.png",
        std::process::id(),
        NEXT_SCRATCH.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    run_ffmpeg(
        &[
            "-i".to_owned(),
            what.path.clone(),
            "-vf".into(),
            format!("scale={PREVIEW_WIDTH}:-1"),
            "-y".into(),
            small.display().to_string(),
        ],
        None,
    )?;
    let bytes = std::fs::read(&small).map_err(|e| format!("no preview was written: {e}"))?;
    let _ = std::fs::remove_file(&small);
    Ok(format!("data:image/png;base64,{}", base64(&bytes)))
}

/// Base64, because one function is cheaper than one dependency.
fn base64(bytes: &[u8]) -> String {
    const SET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(SET[((n >> (18 - i * 6)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
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
        said: None,
    })
}

/// A 16-bit mono PCM WAV around some samples.
///
/// The scripted voice writes one, and so does the envelope's own test: the
/// format is forty-four bytes of header, and having it here means neither
/// needs ffmpeg to make a sound.
pub fn wav(samples: &[i16], rate: u32) -> Vec<u8> {
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let mut out = Vec::with_capacity(44 + data.len());
    out.extend(b"RIFF");
    out.extend(((36 + data.len()) as u32).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes()); // PCM
    out.extend(1u16.to_le_bytes()); // mono
    out.extend(rate.to_le_bytes());
    out.extend((rate * 2).to_le_bytes());
    out.extend(2u16.to_le_bytes());
    out.extend(16u16.to_le_bytes());
    out.extend(b"data");
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(&data);
    out
}

/// How many buckets a lip-sync envelope has per second of audio.
///
/// Twelve is a mouth that moves with the syllables rather than with the
/// waveform: fast enough to look like speech, slow enough that the shell is
/// drawing a mouth rather than an oscilloscope.
pub const ENVELOPE_HZ: usize = 12;

/// The loudness of a sound over time, 0–255 per bucket (SPEC §11.3).
///
/// Lip sync never involves the model: the mouth is driven by the audio that is
/// actually about to play. This reads that audio and hands the shell a shape it
/// can animate, which is a few hundred bytes rather than a few hundred
/// kilobytes — the sound itself never needs to cross the socket for the mouth
/// to move in time with it.
///
/// It reads 16-bit PCM WAV, which is what `audio()` writes and what every
/// text-to-speech in the chain produces. Anything else gets an empty envelope
/// and a closed mouth, which is wrong but quiet — a face that will not lip sync
/// is better than a run that stops.
pub fn envelope(what: &Media) -> Vec<u8> {
    let Ok(bytes) = std::fs::read(&what.path) else {
        return Vec::new();
    };
    // The `data` chunk, found by walking the RIFF chunks rather than assuming
    // it starts at byte 44: ffmpeg writes a LIST chunk before it often enough
    // that the assumption is a bug waiting for a different encoder.
    let (rate, data) = match riff(&bytes) {
        Some(found) => found,
        None => return Vec::new(),
    };
    let samples: Vec<i16> = data
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    if samples.is_empty() || rate == 0 {
        return Vec::new();
    }
    let per_bucket = (rate as usize / ENVELOPE_HZ).max(1);
    samples
        .chunks(per_bucket)
        .map(|bucket| {
            // Peak rather than mean: a mouth follows the loudest thing in the
            // frame, and a mean over a hundredth of a second of speech is
            // mostly the silence between the consonants.
            let peak = bucket
                .iter()
                .map(|s| s.unsigned_abs() as u32)
                .max()
                .unwrap_or(0);
            (peak * 255 / i16::MAX as u32).min(255) as u8
        })
        .collect()
}

/// The sample rate and the `data` chunk of a 16-bit PCM WAV.
fn riff(bytes: &[u8]) -> Option<(u32, &[u8])> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut at = 12;
    let mut rate = 0u32;
    let word = |b: &[u8], at: usize| u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]);
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let size = word(bytes, at + 4) as usize;
        let body = at + 8;
        if body + size > bytes.len() {
            return None;
        }
        match id {
            b"fmt " if size >= 16 => rate = word(bytes, body + 4),
            b"data" => return Some((rate, &bytes[body..body + size])),
            _ => {}
        }
        // Chunks are word-aligned: an odd size is followed by a pad byte.
        at = body + size + (size & 1);
    }
    None
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

    /// A preview is small enough to send with every frame and still a picture.
    #[test]
    fn a_preview_is_a_thumbnail_the_shell_can_show() {
        let scratch = Scratch::open("preview-test").unwrap();
        let into = scratch.file("webcam", "png");
        let media = frame(
            &Input::Synthetic("testsrc=size=1280x720:rate=1".into()),
            None,
            &into,
        )
        .unwrap();

        let uri = preview(&media).unwrap();
        assert!(uri.starts_with("data:image/png;base64,"), "{}", &uri[..40]);
        // A 1280x720 frame is tens of kilobytes; its thumbnail is a few.
        let encoded = uri.len() - "data:image/png;base64,".len();
        assert!(
            encoded > 200,
            "a preview of {encoded} characters is not a picture"
        );
        assert!(
            encoded < media.bytes as usize,
            "the preview ({encoded}) should be smaller than the frame ({})",
            media.bytes
        );
    }

    /// Base64 has to be right, and the awkward part is the padding: one, two
    /// or no bytes left over at the end are three different endings.
    #[test]
    fn base64_pads_the_way_everyone_elses_does() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        // And the bytes above 127 that a PNG is full of.
        assert_eq!(base64(&[0xff, 0xfe, 0xfd]), "//79");
    }

    /// Two captures in one run never collide, however fast they come.
    #[test]
    fn frames_do_not_overwrite_each_other() {
        let scratch = Scratch::open("names-test").unwrap();
        let names: Vec<PathBuf> = (0..50).map(|_| scratch.file("webcam", "png")).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(unique.len(), names.len());
    }

    /// Lip sync is driven by the audio that is about to play, so the envelope
    /// has to actually follow it: a second of silence and a second of tone
    /// must not look the same.
    #[test]
    fn an_envelope_follows_the_sound_it_was_made_from() {
        let scratch = Scratch::open("envelope-test").unwrap();
        let loud = scratch.file("tone", "wav");
        let quiet = scratch.file("hush", "wav");
        audio(&Input::Synthetic("sine=frequency=440".into()), 1.0, &loud).unwrap();
        audio(
            &Input::Synthetic("anullsrc=r=16000:cl=mono".into()),
            1.0,
            &quiet,
        )
        .unwrap();

        let tone = envelope(&Media {
            path: loud.display().to_string(),
            mime: "audio/wav".into(),
            bytes: 0,
            said: None,
        });
        let hush = envelope(&Media {
            path: quiet.display().to_string(),
            mime: "audio/wav".into(),
            bytes: 0,
            said: None,
        });
        assert!(
            tone.len() >= ENVELOPE_HZ - 1 && tone.len() <= ENVELOPE_HZ + 1,
            "a second of audio should be about {ENVELOPE_HZ} buckets, got {}",
            tone.len()
        );
        let peak = *tone.iter().max().unwrap();
        let hushed = *hush.iter().max().unwrap_or(&0);
        // ffmpeg's `sine` is an eighth of full scale, so the number to assert
        // is not "loud" in the abstract — it is that a tone and a silence do
        // not look the same, which is the whole job.
        assert!(peak > 8 * hushed.max(1), "tone {peak}, silence {hushed}");
        assert!(hushed < 8, "silence should be quiet: {hush:?}");
    }

    /// And the scaling itself, against a file this test wrote, so the number is
    /// pinned to arithmetic rather than to whatever amplitude ffmpeg felt like.
    #[test]
    fn full_scale_is_255_and_silence_is_0() {
        let scratch = Scratch::open("scale-test").unwrap();
        let path = scratch.file("made", "wav");
        let rate = 12u32 * 100;
        // The first half is full scale, the second is silence.
        let samples: Vec<i16> = (0..rate)
            .map(|i| if i < rate / 2 { i16::MAX } else { 0 })
            .collect();
        let wav = wav(&samples, rate);
        std::fs::write(&path, &wav).unwrap();

        let shape = envelope(&Media {
            path: path.display().to_string(),
            mime: "audio/wav".into(),
            bytes: wav.len() as u32,
            said: None,
        });
        assert_eq!(shape.len(), ENVELOPE_HZ);
        assert_eq!(shape[0], 255, "full scale should be the top of the range");
        assert_eq!(shape[ENVELOPE_HZ - 1], 0, "silence should be the bottom");
    }

    #[test]
    fn something_that_is_not_a_wav_gets_a_closed_mouth_rather_than_an_error() {
        assert!(
            envelope(&Media {
                path: "/nowhere/at/all.wav".into(),
                mime: "audio/wav".into(),
                bytes: 0,
                said: None,
            })
            .is_empty()
        );
    }
}
