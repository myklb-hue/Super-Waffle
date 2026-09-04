//! A graph that never finishes, driven the way the shell drives it.
//!
//! SPEC §13.2's inbox-triage is the shape: three sources armed, files
//! classified one at a time, a schedule digesting every quarter hour. The
//! acceptance says it runs live for an hour without intervention — an hour is
//! not a thing a test can wait for, so the intervals are turned down and what
//! is proven is the machinery: that a source fires on its own, that only the
//! branch below it runs, that a hold really holds, and that stopping releases
//! everything it took.

use loomd::run::event::{RunEvent, RunOutcome};
use loomd::run::live::Live;
use loomd::run::model::{ChatTurn, Scripted};
use loomd::run::runner::Decision;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

fn fixtures() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures"))
}

/// A scratch folder of this test's own, so two tests never watch each other.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cyberloom-live-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A live run on its own thread, with a handle to stop it.
struct Running {
    events: Receiver<RunEvent>,
    cancel: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<loomd::run::live::Tally>>,
}

impl Running {
    fn start(graph: graph_format::Graph, turns: Vec<ChatTurn>) -> Self {
        let (tx, events) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&cancel);
        let hold = Arc::clone(&paused);
        let root = fixtures();
        let thread = std::thread::spawn(move || {
            let provider = Scripted::new(turns);
            Live {
                graph: &graph,
                root: &root,
                provider: &provider,
                run: "live".into(),
                cancel: stop,
                paused: hold,
                scratch: std::sync::Arc::new(
                    loomd::run::sense::Scratch::open("live-test").unwrap(),
                ),
                eye: std::sync::Arc::new(loomd::run::perceive::Scripted::default()),
            }
            .execute(
                &mut |event| {
                    let _ = tx.send(event);
                },
                &mut |_| Decision::Continue,
            )
        });
        Self {
            events,
            cancel,
            paused,
            thread: Some(thread),
        }
    }

    /// The first event matching a predicate, within a few seconds.
    fn wait(&self, what: &str, ok: impl Fn(&RunEvent) -> bool) -> RunEvent {
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut seen = 0usize;
        while Instant::now() < deadline {
            match self.events.recv_timeout(Duration::from_millis(200)) {
                Ok(event) => {
                    seen += 1;
                    if ok(&event) {
                        return event;
                    }
                }
                Err(_) => continue,
            }
        }
        panic!(
            "never saw {what} in {seen} events; last were {:#?}",
            self.drain()
        );
    }

    /// Whatever is left on the channel, for a failure message that says
    /// something.
    fn drain(&self) -> Vec<String> {
        let mut out = Vec::new();
        while let Ok(event) = self.events.try_recv() {
            out.push(format!("{event:?}"));
        }
        out
    }

    fn quiet_for(&self, how_long: Duration) -> bool {
        let deadline = Instant::now() + how_long;
        while Instant::now() < deadline {
            match self.events.recv_timeout(Duration::from_millis(100)) {
                // Console chatter is not the graph running anything.
                Ok(RunEvent::Console { .. }) => continue,
                Ok(RunEvent::BlockState { .. }) => return false,
                Ok(_) => return false,
                Err(_) => continue,
            }
        }
        true
    }

    fn stop(mut self) -> loomd::run::live::Tally {
        self.cancel.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap()
    }
}

/// A live graph of one Schedule feeding one Terminal: the smallest thing that
/// keeps going on its own.
fn ticking(folder: &std::path::Path, every: &str) -> graph_format::Graph {
    let mut graph = graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap();
    graph.run_mode = graph_format::RunMode::Live;
    graph.blocks = vec![
        block("clock", "schedule", &[("every", every)]),
        block(
            "note",
            "terminal",
            &[
                (
                    "command",
                    &format!("date +%s%N >> {}/ticks", folder.display()),
                ),
                ("warnBefore", "false"),
            ],
        ),
    ];
    graph.wires = vec![graph_format::Wire {
        id: "w1".into(),
        from: graph_format::Endpoint::new("clock", "tick"),
        to: graph_format::Endpoint::new("note", "text"),
    }];
    graph.frames.clear();
    graph
}

fn block(id: &str, kind: &str, settings: &[(&str, &str)]) -> graph_format::Block {
    graph_format::Block {
        id: id.into(),
        kind: kind.into(),
        title: None,
        position: graph_format::Position { x: 0.0, y: 0.0 },
        size: None,
        view: graph_format::View::Summary,
        settings: settings
            .iter()
            .map(|(k, v)| {
                let value = match *v {
                    "true" => graph_format::Setting::Bool(true),
                    "false" => graph_format::Setting::Bool(false),
                    other => match other.parse::<i32>() {
                        Ok(n) => graph_format::Setting::Int(n),
                        Err(_) => graph_format::Setting::String(other.to_owned()),
                    },
                };
                ((*k).to_owned(), value)
            })
            .collect(),
        ports: Vec::new(),
        source: None,
        disabled: false,
        breakpoint: false,
        frame: None,
    }
}

/// A live graph runs itself: nobody presses anything and the work happens.
#[test]
fn a_schedule_keeps_the_graph_going_on_its_own() {
    let folder = scratch("ticking");
    let live = Running::start(ticking(&folder, "0.3s"), vec![]);

    // The source says it is armed before anything fires.
    live.wait("the schedule arming", |e| {
        matches!(e, RunEvent::Console { source: Some(s), message, .. }
            if s == "clock" && message.starts_with("armed"))
    });

    // And then the terminal runs, twice, without anyone asking.
    for _ in 0..2 {
        live.wait(
            "the terminal running",
            |e| matches!(e, RunEvent::BlockDone { block, .. } if block == "note"),
        );
    }

    let tally = live.stop();
    assert!(tally.events >= 2, "{tally:?}");
    assert_eq!(tally.errors, 0);

    let ticks = std::fs::read_to_string(folder.join("ticks")).unwrap();
    assert!(
        ticks.lines().count() >= 2,
        "the command really ran: {ticks:?}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// A hold holds: events keep arriving and nothing runs until it clears
/// (SPEC §8.1).
#[test]
fn pausing_queues_events_and_runs_none_of_them() {
    let folder = scratch("holding");
    let live = Running::start(ticking(&folder, "0.3s"), vec![]);
    live.wait(
        "the first run",
        |e| matches!(e, RunEvent::BlockDone { block, .. } if block == "note"),
    );

    live.paused.store(true, Ordering::Relaxed);
    // Give whatever was already in flight a moment to finish, then watch.
    std::thread::sleep(Duration::from_millis(400));
    while live.events.try_recv().is_ok() {}
    assert!(
        live.quiet_for(Duration::from_millis(1200)),
        "a held graph ran something"
    );
    let during = std::fs::read_to_string(folder.join("ticks")).unwrap_or_default();

    live.paused.store(false, Ordering::Relaxed);
    live.wait(
        "the queue draining",
        |e| matches!(e, RunEvent::BlockDone { block, .. } if block == "note"),
    );

    let tally = live.stop();
    let after = std::fs::read_to_string(folder.join("ticks")).unwrap();
    assert!(
        after.lines().count() > during.lines().count(),
        "the events that queued while it was held ran afterwards"
    );
    // Nothing was thrown away: the queue is a hundred deep and this was a
    // second and a half.
    assert_eq!(tally.dropped, 0, "{tally:?}");
    let _ = std::fs::remove_dir_all(&folder);
}

/// A file arriving runs the triage branch; the quarter-hourly digest is not
/// its business (SPEC §8.2). This is the inbox-triage shape with its
/// intervals turned down.
#[test]
fn one_source_fires_only_what_is_below_it() {
    let inbox = scratch("inbox");
    let mut graph = graph_format::load(fixtures().join("graphs/inbox-triage.loom")).unwrap();
    graph.run_mode = graph_format::RunMode::Live;

    // A webhook would take a port, and a fifteen-minute schedule would not
    // fire inside a test. The folder is the source under examination.
    graph
        .blocks
        .retain(|b| b.kind != "webhook" && b.kind != "schedule");
    graph
        .wires
        .retain(|w| w.from.node != "webhook" && w.from.node != "schedule");
    let watch = graph.blocks.iter_mut().find(|b| b.id == "watch").unwrap();
    watch.settings.insert(
        "path".into(),
        graph_format::Setting::String(inbox.display().to_string()),
    );
    watch
        .settings
        .insert("debounce".into(), graph_format::Setting::Int(0));

    // The classifier answers; nothing else needs a model.
    let live = Running::start(
        graph,
        vec![Scripted::says("urgent"), Scripted::says("urgent")],
    );
    live.wait("the folder arming", |e| {
        matches!(e, RunEvent::Console { source: Some(s), message, .. }
            if s == "watch" && message.starts_with("armed"))
    });

    std::fs::write(inbox.join("first.eml"), "a message").unwrap();

    // The classifier ran, and it was given the file that arrived.
    live.wait("the classifier", |e| {
        matches!(e, RunEvent::BlockState { block, state, .. }
            if block == "classify" && *state == loomd::run::event::BlockState::Running)
    });

    let tally = live.stop();
    assert_eq!(tally.events, 1, "one file is one event: {tally:?}");
    let _ = std::fs::remove_dir_all(&inbox);
}

/// Stopping releases what the sources took. A webhook that kept its port would
/// make the second run of the same graph fail with "address already in use",
/// which is the kind of thing that only shows up on the second run.
#[test]
fn stopping_hands_back_the_port_so_the_graph_can_run_again() {
    let port = 7000 + (std::process::id() % 900) as i32;
    let build = || {
        let mut graph = graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap();
        graph.run_mode = graph_format::RunMode::Live;
        graph.blocks = vec![block(
            "hook",
            "webhook",
            &[("port", &port.to_string()), ("path", "/in")],
        )];
        graph.wires.clear();
        graph.frames.clear();
        graph
    };

    for attempt in 1..=2 {
        let live = Running::start(build(), vec![]);
        live.wait(&format!("the webhook arming on attempt {attempt}"), |e| {
            matches!(e, RunEvent::Console { source: Some(s), message, .. }
                if s == "hook" && message.starts_with("armed"))
        });
        // It is really listening.
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port as u16)).unwrap();
        write!(
            stream,
            "POST /in HTTP/1.1\r\nHost: x\r\nContent-Length: 2\r\n\r\n{{}}"
        )
        .unwrap();
        stream.flush().unwrap();
        live.stop();
    }
}

/// A live graph with nothing armed says so rather than sitting there looking
/// like it is working.
#[test]
fn a_live_graph_with_no_source_says_so() {
    let mut graph = graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap();
    graph.run_mode = graph_format::RunMode::Live;
    let live = Running::start(graph, vec![]);

    let complaint = live.wait(
        "the complaint",
        |e| matches!(e, RunEvent::Console { message, .. } if message.contains("needs a source")),
    );
    let RunEvent::Console { level, .. } = complaint else {
        unreachable!()
    };
    assert_eq!(level, loomd::run::event::Level::Error);

    live.wait(
        "it giving up",
        |e| matches!(e, RunEvent::Finished { outcome, .. } if *outcome == RunOutcome::Failed),
    );
    live.stop();
}

/// A camera is a source, and a live graph has to arm it like one.
///
/// This was a hole rather than a subtlety: `webcam` said `source: true` in the
/// catalogue, so a graph holding one was live, and nothing armed it — the graph
/// came up, reported nothing armed, and sat there. Every capture in the engine
/// had a test and none of them was live.
///
/// `lavfi:` stands in for a camera the way it does throughout: the same code
/// path that opens `/dev/video0`, with a device this machine actually has.
#[test]
fn a_live_camera_captures_on_its_own() {
    let dir = scratch("camera");
    let mut graph = graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap();
    graph.run_mode = graph_format::RunMode::Live;
    graph.blocks = vec![
        block(
            "eye",
            "webcam",
            &[
                ("device", "lavfi:testsrc"),
                ("resolution", "160x120"),
                ("fps", "4"),
            ],
        ),
        block(
            "note",
            "terminal",
            &[
                (
                    "command",
                    &format!("date +%s%N >> {}/frames", dir.display()),
                ),
                ("warnBefore", "false"),
            ],
        ),
    ];
    graph.wires = vec![graph_format::Wire {
        id: "w1".into(),
        from: graph_format::Endpoint::new("eye", "frames"),
        to: graph_format::Endpoint::new("note", "text"),
    }];
    graph.frames.clear();
    let live = Running::start(graph, vec![]);

    live.wait(
        "the camera arming",
        |e| matches!(e, RunEvent::SourceArmed { block, state, .. } if block == "eye" && state.contains("4 fps")),
    );

    // The camera itself ran, and what came off it is a frame rather than the
    // `null` a tick carries.
    let done = live.wait(
        "a frame",
        |e| matches!(e, RunEvent::BlockDone { block, .. } if block == "eye"),
    );
    let RunEvent::BlockDone { outputs, .. } = &done else {
        unreachable!()
    };
    let frame = outputs.iter().find(|p| p.port == "frames").expect("frames");
    assert!(
        matches!(&frame.value, loomd::run::value::Value::Image(m) if m.mime == "image/png"),
        "the camera put {:?} on its wire, not a picture",
        frame.value
    );

    // And it keeps going: a source fires more than once.
    live.wait(
        "a second frame",
        |e| matches!(e, RunEvent::BlockDone { block, .. } if block == "eye"),
    );
    live.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// A camera that is not there holds the graph rather than flooding it.
///
/// SPEC §12.1: "a hardware fault *pauses*; one click resumes". Without it a
/// missing camera reports the same failure at the camera's tick rate, forever
/// — which is how a console becomes useless at the exact moment it is needed.
#[test]
fn a_camera_that_is_not_there_holds_the_graph() {
    let mut graph = graph_format::load(fixtures().join("graphs/customer-triage.loom")).unwrap();
    graph.run_mode = graph_format::RunMode::Live;
    graph.blocks = vec![
        block(
            "eye",
            "webcam",
            &[("device", "/dev/video-nothing-is-here"), ("fps", "20")],
        ),
        block("note", "output", &[]),
    ];
    graph.wires = vec![graph_format::Wire {
        id: "w1".into(),
        from: graph_format::Endpoint::new("eye", "frames"),
        to: graph_format::Endpoint::new("note", "value"),
    }];
    graph.frames.clear();
    let live = Running::start(graph, vec![]);

    live.wait(
        "the hold",
        |e| matches!(e, RunEvent::Held { held, .. } if *held),
    );

    // And it stays held: the flood is what pausing is for, so nothing may run
    // after it. Twenty ticks a second means a graph still going would be
    // unmistakable within half a second.
    assert!(
        live.quiet_for(Duration::from_millis(700)),
        "the graph kept running after the fault held it"
    );
    live.stop();
}
