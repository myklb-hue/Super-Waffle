//! Slice 7's acceptance: a webcam frame reaches an LLM as a detection.
//!
//! Every step of that sentence is real here except the model weights. The
//! frame is captured by ffmpeg — there is no camera on this machine, which is
//! precisely what `lavfi:` is for — and it is a real PNG on disk with real
//! bytes. The detection is scripted, because whether yolo-v8n finds a door is
//! a question about yolo-v8n. What the engine does with the answer is not, and
//! that is what these check.

use loomd::run::event::{RunEvent, RunOutcome};
use loomd::run::model::{ChatTurn, Scripted as ScriptedModel};
use loomd::run::perceive::{Affect, Perception, Person, Scripted as ScriptedEye};
use loomd::run::runner::{Decision, Runner, Summary};
use loomd::run::sense::Scratch;
use loomd::run::value::Value;
use std::sync::Arc;

fn fixtures() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures"))
}

fn block(id: &str, kind: &str, settings: &[(&str, graph_format::Setting)]) -> graph_format::Block {
    graph_format::Block {
        id: id.into(),
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

fn wire(id: &str, from: (&str, &str), to: (&str, &str)) -> graph_format::Wire {
    graph_format::Wire {
        id: id.into(),
        from: graph_format::Endpoint::new(from.0, from.1),
        to: graph_format::Endpoint::new(to.0, to.1),
    }
}

fn text(s: &str) -> graph_format::Setting {
    graph_format::Setting::String(s.into())
}

struct Ran {
    summary: Summary,
    events: Vec<RunEvent>,
    /// Held for the length of the assertions.
    ///
    /// A run's frames go away with the run — that is the privacy default, and
    /// `sense` has its own test for it. Here the folder has to outlive
    /// `execute` so the captures can be looked at, which is exactly what the
    /// daemon does: the scratch belongs to the run, and the run is not over
    /// until the session forgets it.
    _scratch: Arc<Scratch>,
}

impl Ran {
    fn done(&self, block: &str) -> Option<&Vec<loomd::run::event::PortValue>> {
        self.events.iter().find_map(|e| match e {
            RunEvent::BlockDone {
                block: b, outputs, ..
            } if b == block => Some(outputs),
            _ => None,
        })
    }

    fn figure(&self, block: &str) -> Option<String> {
        self.events.iter().find_map(|e| match e {
            RunEvent::BlockDone {
                block: b, figure, ..
            } if b == block => figure.clone(),
            _ => None,
        })
    }
}

fn run(graph: &graph_format::Graph, eye: Arc<dyn Perception>, turns: Vec<ChatTurn>) -> Ran {
    let provider = ScriptedModel::new(turns);
    let root = fixtures();
    let mut events = Vec::new();
    let scratch = Arc::new(Scratch::open("senses-test").unwrap());
    let summary = Runner {
        graph,
        root: &root,
        provider: &provider,
        run: "senses".into(),
        cancel: Default::default(),
        scratch: Arc::clone(&scratch),
        eye,
        vault: Arc::new(loomd::run::memory::Vault::new(&root)),
        bench: Arc::new(loomd::run::runner::Bench::scripted()),
    }
    .execute(&mut |e| events.push(e), &mut |_| Decision::Continue);
    Ran {
        summary,
        events,
        _scratch: scratch,
    }
}

/// Webcam → Object detection → LLM → Output, and what comes out the far end
/// has a door in it.
#[test]
fn a_webcam_frame_reaches_a_model_as_a_detection() {
    let mut graph = graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap();
    graph.frames.clear();
    graph.blocks = vec![
        block(
            "webcam",
            "webcam",
            &[
                ("device", text("lavfi:testsrc=size=320x240:rate=1")),
                ("resolution", text("320x240")),
            ],
        ),
        block("eye", "object-detection", &[("model", text("yolo-v8n"))]),
        block(
            "llm",
            "llm",
            &[(
                "systemPrompt",
                text("Say what you were told is in the frame."),
            )],
        ),
        block("report", "output", &[("name", text("report"))]),
    ];
    graph.wires = vec![
        wire("w1", ("webcam", "frames"), ("eye", "image")),
        wire("w2", ("eye", "objects"), ("llm", "prompt")),
        wire("w3", ("llm", "text"), ("report", "value")),
    ];

    let eye = Arc::new(ScriptedEye::sees(&[("door", 0.88), ("person", 0.41)]));
    let ran = run(
        &graph,
        eye.clone(),
        vec![ScriptedModel::says("The front door is open.")],
    );
    assert_eq!(
        ran.summary.outcome,
        RunOutcome::Finished,
        "{:?}",
        ran.summary.errors
    );

    // The frame is real: ffmpeg captured it, and it is a PNG with bytes in it.
    let frame = ran.done("webcam").expect("the webcam produced a frame");
    let Value::Image(media) = &frame[0].value else {
        panic!("a webcam produces an image, not {:?}", frame[0].value);
    };
    assert_eq!(media.mime, "image/png");
    assert!(media.bytes > 500, "{} bytes", media.bytes);
    assert!(std::path::Path::new(&media.path).exists());

    // The detector was given that exact file.
    assert_eq!(
        eye.seen.lock().unwrap().as_slice(),
        [format!("detect {}", media.path)]
    );

    // And the model was told what was in it, in words it could read.
    let prompt = ran
        .done("eye")
        .expect("the detector produced objects")
        .first()
        .map(|p| p.value.as_text())
        .unwrap();
    assert!(prompt.contains("door"), "{prompt}");

    let answer = &ran.summary.results;
    assert_eq!(answer.len(), 1);
    assert_eq!(
        answer[0].value,
        Value::Text("The front door is open.".into())
    );

    // The block said what it saw, inline, while it was running (SPEC §3.2).
    assert_eq!(ran.figure("eye").as_deref(), Some("2 · door"));
}

/// A frame goes into the run's own folder and nowhere else, unless recording
/// is on (SPEC §12.3).
#[test]
fn a_frame_is_not_recorded_unless_recording_is_on() {
    let mut graph = graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap();
    graph.frames.clear();
    graph.blocks = vec![block(
        "webcam",
        "webcam",
        &[("device", text("lavfi:testsrc=rate=1"))],
    )];
    graph.wires.clear();

    let ran = run(&graph, Arc::new(ScriptedEye::default()), vec![]);
    let frame = ran.done("webcam").unwrap();
    let Value::Image(media) = &frame[0].value else {
        panic!("expected an image")
    };
    // In the run's scratch, which the run takes with it when it ends.
    assert!(
        media.path.contains("cyberloom-run-"),
        "a frame nobody asked to keep went to {}",
        media.path
    );
    assert!(
        !media.path.contains("recordings"),
        "a frame was recorded without being asked"
    );
}

/// And with recording on it is copied somewhere durable, and the console says
/// so — recording is not a thing that happens quietly.
#[test]
fn recording_says_where_the_frame_went() {
    let mut graph = graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap();
    graph.frames.clear();
    graph.blocks = vec![block(
        "webcam",
        "webcam",
        &[
            ("device", text("lavfi:testsrc=rate=1")),
            ("store", graph_format::Setting::Bool(true)),
        ],
    )];
    graph.wires.clear();

    let ran = run(&graph, Arc::new(ScriptedEye::default()), vec![]);
    let frame = ran.done("webcam").unwrap();
    let Value::Image(media) = &frame[0].value else {
        panic!("expected an image")
    };
    assert!(media.path.contains("recordings"), "{}", media.path);
    assert!(std::path::Path::new(&media.path).exists());

    let said = ran.events.iter().any(|e| {
        matches!(e, RunEvent::Console { source: Some(s), message, .. }
            if s == "webcam" && message.starts_with("recorded to"))
    });
    assert!(said, "recording happened without saying so");

    let _ = std::fs::remove_dir_all(fixtures().join("recordings"));
}

/// A microphone captures real audio, and speech-to-text reads it.
#[test]
fn a_microphone_reaches_speech_to_text() {
    let mut graph = graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap();
    graph.frames.clear();
    graph.blocks = vec![
        block(
            "mic",
            "microphone",
            &[
                ("device", text("lavfi:sine=frequency=440")),
                ("seconds", graph_format::Setting::Float(0.3)),
            ],
        ),
        block("ears", "speech-to-text", &[("model", text("whisper-base"))]),
        block("said", "output", &[("name", text("said"))]),
    ];
    graph.wires = vec![
        wire("w1", ("mic", "audio"), ("ears", "audio")),
        wire("w2", ("ears", "text"), ("said", "value")),
    ];

    let eye = Arc::new(ScriptedEye {
        transcript: Some(loomd::run::perceive::Heard {
            text: "open the front door".into(),
            seconds: 0.3,
        }),
        ..Default::default()
    });
    let ran = run(&graph, eye, vec![]);
    assert_eq!(
        ran.summary.outcome,
        RunOutcome::Finished,
        "{:?}",
        ran.summary.errors
    );
    assert_eq!(
        ran.summary.results[0].value,
        Value::Text("open the front door".into())
    );
    // The lag figure §6.1 asks for.
    assert_eq!(ran.figure("ears").as_deref(), Some("0.3s"));
}

/// Affect carries the expression, so an Avatar does not have to know the
/// thresholds (SPEC §11.2).
#[test]
fn affect_hands_on_a_face_and_not_just_two_numbers() {
    let mut graph = graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap();
    graph.frames.clear();
    graph.blocks = vec![
        block(
            "input",
            "input",
            &[("value", text("this is wonderful news"))],
        ),
        block("mood", "affect", &[]),
        block("out", "output", &[("name", text("mood"))]),
    ];
    graph.wires = vec![
        wire("w1", ("input", "value"), ("mood", "text")),
        wire("w2", ("mood", "affect"), ("out", "value")),
    ];

    let eye = Arc::new(ScriptedEye {
        mood: Some(Affect {
            valence: 0.7,
            arousal: 0.4,
        }),
        ..Default::default()
    });
    let ran = run(&graph, eye, vec![]);
    let Value::Data(json) = &ran.summary.results[0].value else {
        panic!("affect is a record")
    };
    assert_eq!(json["express"], "smile");
    assert_eq!(json["valence"], 0.7);
    assert_eq!(ran.figure("mood").as_deref(), Some("smile"));
}

/// A face reaches the graph as a name and a confidence, never as an image
/// (SPEC §12.3). The value on the wire is what a graph file would hold.
#[test]
fn a_recognised_face_puts_no_image_on_the_wire() {
    let mut graph = graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap();
    graph.frames.clear();
    graph.blocks = vec![
        block(
            "webcam",
            "webcam",
            &[("device", text("lavfi:testsrc=rate=1"))],
        ),
        block("who", "face-recognition", &[]),
        block("out", "output", &[("name", text("who"))]),
    ];
    graph.wires = vec![
        wire("w1", ("webcam", "frames"), ("who", "image")),
        wire("w2", ("who", "person"), ("out", "value")),
    ];

    let eye = Arc::new(ScriptedEye {
        person: Some(Person {
            name: Some("mykl".into()),
            confidence: 0.91,
            dimensions: 512,
        }),
        ..Default::default()
    });
    let ran = run(&graph, eye, vec![]);
    let Value::Data(json) = &ran.summary.results[0].value else {
        panic!("a person is a record")
    };
    assert_eq!(json["name"], "mykl");
    assert_eq!(json["dimensions"], 512);

    // Nothing in it points at a file, which is the property §12.3 asks for.
    let flat = json.to_string();
    assert!(!flat.contains(".png"), "an image reached the wire: {flat}");
    assert!(!flat.contains("/tmp"), "a path reached the wire: {flat}");
}

/// A camera that is not there is reported in words that name the fix, and the
/// rest of the graph carries on (SPEC §12.1).
#[test]
fn a_missing_camera_is_reported_and_the_graph_survives() {
    let mut graph = graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap();
    graph.frames.clear();
    graph.blocks = vec![
        block("webcam", "webcam", &[("device", text("/dev/video-nope"))]),
        block("elsewhere", "input", &[("value", text("still here"))]),
        block("out", "output", &[("name", text("out"))]),
    ];
    graph.wires = vec![wire("w1", ("elsewhere", "value"), ("out", "value"))];

    let ran = run(&graph, Arc::new(ScriptedEye::default()), vec![]);
    assert_eq!(ran.summary.outcome, RunOutcome::Failed);

    let why = ran
        .events
        .iter()
        .find_map(|e| match e {
            RunEvent::BlockError { block, message, .. } if block == "webcam" => Some(message),
            _ => None,
        })
        .expect("the camera reported");
    assert!(why.contains("lavfi:testsrc"), "{why}");

    // The branch that had nothing to do with the camera still ran.
    assert_eq!(
        ran.summary.results[0].value,
        Value::Text("still here".into())
    );
}
