//! Blocks that emit on their own initiative (SPEC §6.4, §8.2).
//!
//! A source is what makes a graph *live*: it is armed rather than run, it fires
//! when something happens in the world, and the graph downstream of it runs
//! once per firing. A graph holding one never finishes on its own.
//!
//! Each source owns a thread and pushes into one channel. Threads rather than
//! an async runtime for the same reason the rest of the engine has none: there
//! are three of these per graph, not three thousand, and a thread that sleeps
//! is exactly as cheap as a task that awaits at this scale — while being
//! something you can read top to bottom.

use super::value::Value;
use graph_format::Block;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant, SystemTime};

/// One thing a source noticed.
#[derive(Debug, Clone)]
pub struct Fired {
    /// The block that noticed it.
    pub node: String,
    /// The port it came out of.
    pub port: String,
    pub value: Value,
    pub at: Instant,
}

/// A source that is running. Dropping this stops it.
pub struct Armed {
    pub node: String,
    /// What the block's header chip says: `armed`, `listening`, `every 15m`.
    pub state: String,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Armed {
    /// Ask it to stop and wait for it.
    ///
    /// The wait matters: a webhook holding a port has to release it before the
    /// same graph can be started again, and a silent race there looks like
    /// "address already in use" on the second run.
    pub fn disarm(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Armed {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// How often a polling source looks.
///
/// Watching a folder is done by looking rather than by asking the kernel. The
/// honest trade: inotify would notice sooner and would add a dependency and a
/// platform, and an inbox that is noticed within half a second is noticed soon
/// enough. The debounce a folder source already needs would swallow most of
/// the difference anyway.
const POLL: Duration = Duration::from_millis(500);

/// Arm one source block, if it is one.
pub fn arm(block: &Block, root: &Path, out: Sender<Fired>) -> Result<Option<Armed>, String> {
    let stop = Arc::new(AtomicBool::new(false));
    let id = block.id.clone();
    let flag = Arc::clone(&stop);

    let (state, thread) = match block.kind.as_str() {
        "schedule" => {
            let every = super::blocks::setting(block, "every")
                .ok_or("the Schedule has no interval: set Every to something like `15m`")?;
            let period = parse_every(every)
                .ok_or_else(|| format!("`{every}` is not an interval; try `30s`, `15m` or `2h`"))?;
            let jitter =
                Duration::from_secs_f64(super::blocks::number(block, "jitter").unwrap_or(0.0));
            let node = id.clone();
            (
                format!("every {every}"),
                std::thread::Builder::new()
                    .name(format!("loomd-schedule-{id}"))
                    .spawn(move || schedule(&node, period, jitter, &flag, &out))
                    .map_err(|e| e.to_string())?,
            )
        }

        "watch-folder" => {
            let path = super::blocks::setting(block, "path")
                .ok_or("the Watch folder has no path")?
                .to_owned();
            let folder = expand(&path, root);
            let pattern = super::blocks::setting(block, "pattern")
                .unwrap_or("*")
                .to_owned();
            let debounce = Duration::from_millis(
                super::blocks::number(block, "debounce").unwrap_or(0.0) as u64,
            );
            // What is already there is taken *now*, before the thread starts
            // and before the block reports itself armed. Listing inside the
            // thread left a window between "armed" and the first look in which
            // a file could arrive, be counted as already present, and never be
            // reported — silently, forever. The window is small and the bug it
            // hides is not.
            let already = list(&folder, &pattern);
            let node = id.clone();
            (
                format!("watching {path}"),
                std::thread::Builder::new()
                    .name(format!("loomd-watch-{id}"))
                    .spawn(move || watch(&node, &folder, &pattern, debounce, already, &flag, &out))
                    .map_err(|e| e.to_string())?,
            )
        }

        "webhook" => {
            let port =
                super::blocks::number(block, "port").ok_or("the Webhook has no port")? as u16;
            let path = super::blocks::setting(block, "path")
                .unwrap_or("/")
                .to_owned();
            let listener = std::net::TcpListener::bind(("127.0.0.1", port))
                .map_err(|e| format!("could not listen on {port}: {e}"))?;
            // A blocking accept would never see the stop flag. A short timeout
            // makes the loop check it between connections without spinning.
            listener
                .set_nonblocking(true)
                .map_err(|e| format!("could not arm the listener: {e}"))?;
            let node = id.clone();
            (
                format!("listening on :{port}{path}"),
                std::thread::Builder::new()
                    .name(format!("loomd-webhook-{id}"))
                    .spawn(move || webhook(&node, listener, &path, &flag, &out))
                    .map_err(|e| e.to_string())?,
            )
        }

        // Not a source: a webcam and a microphone are, but they are devices and
        // belong with the rest of the senses.
        _ => return Ok(None),
    };

    Ok(Some(Armed {
        node: id,
        state,
        stop,
        thread: Some(thread),
    }))
}

/// `30s`, `15m`, `2h`, `1d`, or a bare number of seconds.
pub fn parse_every(text: &str) -> Option<Duration> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let (digits, unit) = text.split_at(text.len() - 1);
    let (value, seconds): (f64, f64) = match unit {
        "s" => (digits.parse().ok()?, 1.0),
        "m" => (digits.parse().ok()?, 60.0),
        "h" => (digits.parse().ok()?, 3600.0),
        "d" => (digits.parse().ok()?, 86400.0),
        _ => (text.parse().ok()?, 1.0),
    };
    (value > 0.0).then(|| Duration::from_secs_f64(value * seconds))
}

/// Sleep in short steps so a stop is noticed promptly.
///
/// A fifteen-minute schedule that slept fifteen minutes would take fifteen
/// minutes to stop, and a Stop button that takes a quarter of an hour is not
/// a Stop button.
fn nap(total: Duration, stop: &AtomicBool) -> bool {
    let step = Duration::from_millis(100);
    let mut left = total;
    while left > Duration::ZERO {
        if stop.load(Ordering::Relaxed) {
            return false;
        }
        let this = step.min(left);
        std::thread::sleep(this);
        left -= this;
    }
    !stop.load(Ordering::Relaxed)
}

fn schedule(
    node: &str,
    period: Duration,
    jitter: Duration,
    stop: &AtomicBool,
    out: &Sender<Fired>,
) {
    let mut tick = 0u64;
    loop {
        // Jitter spreads several schedules that would otherwise fire together
        // (SPEC §6.4). Deterministic from the tick count rather than random:
        // it does the spreading, and a run that logs the same times twice is
        // easier to argue with than one that does not.
        let spread = if jitter.is_zero() {
            Duration::ZERO
        } else {
            let fraction = ((tick.wrapping_mul(2654435761) >> 8) % 1000) as f64 / 1000.0;
            jitter.mul_f64(fraction)
        };
        if !nap(period + spread, stop) {
            return;
        }
        tick += 1;
        // A tick carries no value: `tick` is an exec port, and exec is control
        // flow rather than a value (SPEC §4.3).
        if out
            .send(Fired {
                node: node.to_owned(),
                port: "tick".into(),
                value: Value::Null,
                at: Instant::now(),
            })
            .is_err()
        {
            return;
        }
    }
}

fn watch(
    node: &str,
    folder: &Path,
    pattern: &str,
    debounce: Duration,
    // What was already there when the block armed is not an event. A folder
    // source reports what arrives, not what it found — and the listing is
    // taken by the caller, so nothing can slip into the gap.
    mut seen: std::collections::BTreeMap<PathBuf, SystemTime>,
    stop: &AtomicBool,
    out: &Sender<Fired>,
) {
    loop {
        if !nap(POLL, stop) {
            return;
        }
        let now = list(folder, pattern);
        for (path, modified) in &now {
            let is_new = match seen.get(path) {
                None => true,
                Some(before) => before != modified,
            };
            if !is_new {
                continue;
            }
            // Debounce: a file still being written changes size between polls,
            // and reporting it half-copied is worse than reporting it late.
            if !debounce.is_zero() {
                let settled = modified
                    .elapsed()
                    .map(|since| since >= debounce)
                    .unwrap_or(false);
                if !settled {
                    continue;
                }
            }
            seen.insert(path.clone(), *modified);
            if out
                .send(Fired {
                    node: node.to_owned(),
                    port: "file".into(),
                    value: Value::File(path.display().to_string()),
                    at: Instant::now(),
                })
                .is_err()
            {
                return;
            }
        }
        // A file that went away should fire again if it comes back.
        seen.retain(|path, _| now.contains_key(path));
    }
}

fn list(folder: &Path, pattern: &str) -> std::collections::BTreeMap<PathBuf, SystemTime> {
    let mut out = std::collections::BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(folder) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !matches_glob(name, pattern) {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        out.insert(path, modified);
    }
    out
}

/// `*.eml`, `report-*`, `*`. Enough of a glob for a folder pattern, and no
/// more: a full matcher is a dependency for a feature nobody asked for.
pub fn matches_glob(name: &str, pattern: &str) -> bool {
    let mut parts = pattern.split('*');
    let Some(first) = parts.next() else {
        return true;
    };
    if !name.starts_with(first) {
        return false;
    }
    let mut at = first.len();
    let mut last: Option<&str> = None;
    for part in parts {
        last = Some(part);
        if part.is_empty() {
            continue;
        }
        match name[at..].find(part) {
            Some(found) => at += found + part.len(),
            None => return false,
        }
    }
    match last {
        // A pattern ending in a literal has to end the name too.
        Some(tail) if !tail.is_empty() => name.ends_with(tail) && at <= name.len(),
        // A pattern ending in `*` matches whatever is left.
        Some(_) => true,
        // No `*` at all: an exact name.
        None => name == pattern,
    }
}

fn webhook(
    node: &str,
    listener: std::net::TcpListener,
    path: &str,
    stop: &AtomicBool,
    out: &Sender<Fired>,
) {
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                if let Some(body) = read_request(stream, path) {
                    let value = serde_json::from_str::<serde_json::Value>(&body)
                        .map(Value::Data)
                        .unwrap_or(Value::Text(body));
                    if out
                        .send(Fired {
                            node: node.to_owned(),
                            port: "event".into(),
                            value,
                            at: Instant::now(),
                        })
                        .is_err()
                    {
                        return;
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return,
        }
    }
}

/// Read one request and answer it. Returns the body when the path matches.
///
/// A hand-written reader rather than an HTTP crate: what this has to
/// understand is a request line, a content length and a body, and the whole of
/// it fits on a screen. A server that accepted more than that would be
/// accepting more than a webhook needs.
fn read_request(mut stream: std::net::TcpStream, want: &str) -> Option<String> {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut reader = BufReader::new(stream.try_clone().ok()?);

    let mut request_line = String::new();
    reader.read_line(&mut request_line).ok()?;
    let mut parts = request_line.split_whitespace();
    let _method = parts.next()?;
    let path = parts.next().unwrap_or("/");
    // The query string is not part of the path a webhook is bound to.
    let path = path.split('?').next().unwrap_or(path);

    let mut length = 0usize;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).ok()? == 0 {
            break;
        }
        if header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap_or(0);
        }
    }

    let mut body = vec![0u8; length];
    if length > 0 {
        reader.read_exact(&mut body).ok()?;
    }

    let matched = path == want || (want == "/" && path.is_empty());
    let status = if matched { "200 OK" } else { "404 Not Found" };
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let _ = stream.flush();

    matched.then(|| String::from_utf8_lossy(&body).into_owned())
}

/// `~/inbox` against the user's home, anything relative against the workspace.
pub fn expand(path: &str, root: &Path) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

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
    fn an_interval_reads_the_way_it_is_written() {
        assert_eq!(parse_every("30s"), Some(Duration::from_secs(30)));
        assert_eq!(parse_every("15m"), Some(Duration::from_secs(900)));
        assert_eq!(parse_every("2h"), Some(Duration::from_secs(7200)));
        assert_eq!(parse_every("1d"), Some(Duration::from_secs(86400)));
        // A bare number is seconds.
        assert_eq!(parse_every("45"), Some(Duration::from_secs(45)));
        // Nothing that would arm a schedule firing forever.
        assert_eq!(parse_every("0s"), None);
        assert_eq!(parse_every(""), None);
        assert_eq!(parse_every("soon"), None);
    }

    #[test]
    fn a_glob_matches_what_a_folder_pattern_needs() {
        assert!(matches_glob("mail.eml", "*.eml"));
        assert!(!matches_glob("mail.txt", "*.eml"));
        assert!(matches_glob("report-2026.csv", "report-*"));
        assert!(matches_glob("anything", "*"));
        assert!(matches_glob("exact.eml", "exact.eml"));
        assert!(!matches_glob("other.eml", "exact.eml"));
        // A star in the middle.
        assert!(matches_glob("a-big-file.eml", "a-*.eml"));
        assert!(!matches_glob("b-big-file.eml", "a-*.eml"));
    }

    /// A schedule fires on its own, and stops promptly when asked.
    #[test]
    fn a_schedule_fires_and_then_stops() {
        let (tx, rx) = channel();
        let armed = arm(
            &block(
                "schedule",
                &[("every", graph_format::Setting::String("0.2s".into()))],
            ),
            Path::new("/tmp"),
            tx,
        )
        .unwrap()
        .expect("a schedule is a source");
        assert_eq!(armed.state, "every 0.2s");

        let first = rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(first.node, "schedule");
        assert_eq!(first.port, "tick");
        // A tick is control flow, not a value (SPEC §4.3).
        assert_eq!(first.value, Value::Null);

        let stopping = Instant::now();
        armed.disarm();
        assert!(
            stopping.elapsed() < Duration::from_secs(1),
            "a stop should not wait out the interval"
        );
    }

    /// A schedule with no interval says what to write rather than arming and
    /// never firing.
    #[test]
    fn a_schedule_with_no_interval_says_what_to_set() {
        let Err(error) = arm(&block("schedule", &[]), Path::new("/tmp"), channel().0) else {
            panic!("a schedule with no interval cannot arm");
        };
        assert!(error.contains("15m"), "{error}");
    }

    /// A folder source reports what arrives, not what it found.
    #[test]
    fn a_folder_reports_what_arrives_and_not_what_was_there() {
        let dir = std::env::temp_dir().join(format!("cyberloom-watch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("already.eml"), "old").unwrap();

        let (tx, rx) = channel();
        let armed = arm(
            &block(
                "watch-folder",
                &[
                    (
                        "path",
                        graph_format::Setting::String(dir.display().to_string()),
                    ),
                    ("pattern", graph_format::Setting::String("*.eml".into())),
                ],
            ),
            Path::new("/tmp"),
            tx,
        )
        .unwrap()
        .unwrap();

        // Nothing for the file that was already there.
        assert!(rx.recv_timeout(Duration::from_millis(900)).is_err());

        std::fs::write(dir.join("new.eml"), "hello").unwrap();
        let _ = &armed;
        std::fs::write(dir.join("ignored.txt"), "not a mail").unwrap();

        let fired = rx.recv_timeout(Duration::from_secs(4)).unwrap();
        assert_eq!(fired.port, "file");
        assert_eq!(
            fired.value,
            Value::File(dir.join("new.eml").display().to_string())
        );
        // And the file that does not match the pattern never arrives.
        assert!(rx.recv_timeout(Duration::from_millis(900)).is_err());

        armed.disarm();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A file that lands in the instant between arming and the first look is
    /// still an arrival.
    ///
    /// The listing used to be taken inside the source's own thread, which left
    /// a window: a file written just after the block said "armed" was counted
    /// as already present and never reported at all. Writing immediately is
    /// what reopens that window if it ever comes back.
    #[test]
    fn a_file_arriving_the_instant_it_arms_is_not_missed() {
        let dir = std::env::temp_dir().join(format!("cyberloom-race-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let (tx, rx) = channel();
        let armed = arm(
            &block(
                "watch-folder",
                &[(
                    "path",
                    graph_format::Setting::String(dir.display().to_string()),
                )],
            ),
            Path::new("/tmp"),
            tx,
        )
        .unwrap()
        .unwrap();

        // No pause at all: this is the window.
        std::fs::write(dir.join("instant.txt"), "hello").unwrap();

        let fired = rx.recv_timeout(Duration::from_secs(4)).unwrap();
        assert_eq!(
            fired.value,
            Value::File(dir.join("instant.txt").display().to_string())
        );
        armed.disarm();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A webhook takes a POST and hands on its body.
    #[test]
    fn a_webhook_turns_a_post_into_an_event() {
        let (tx, rx) = channel();
        // Port 0 would be chosen by the kernel, which the block cannot report,
        // so this picks one high enough to be free.
        let port = 8000 + (std::process::id() % 1000) as i32;
        let armed = arm(
            &block(
                "webhook",
                &[
                    ("port", graph_format::Setting::Int(port)),
                    ("path", graph_format::Setting::String("/inbox".into())),
                ],
            ),
            Path::new("/tmp"),
            tx,
        )
        .unwrap()
        .unwrap();
        assert!(armed.state.contains("/inbox"), "{}", armed.state);

        let body = r#"{"from":"a@example.com","subject":"urgent"}"#;
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port as u16)).unwrap();
        write!(
            stream,
            "POST /inbox HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        stream.flush().unwrap();

        let fired = rx.recv_timeout(Duration::from_secs(4)).unwrap();
        assert_eq!(fired.port, "event");
        let Value::Data(json) = fired.value else {
            panic!("a webhook body that is JSON arrives as a record");
        };
        assert_eq!(json["subject"], "urgent");
        armed.disarm();
    }

    /// A request to another path is answered and ignored, so a webhook bound
    /// to `/inbox` is not fired by a health check on `/`.
    #[test]
    fn a_webhook_ignores_another_path() {
        let (tx, rx) = channel();
        let port = 9000 + (std::process::id() % 1000) as i32;
        let armed = arm(
            &block(
                "webhook",
                &[
                    ("port", graph_format::Setting::Int(port)),
                    ("path", graph_format::Setting::String("/inbox".into())),
                ],
            ),
            Path::new("/tmp"),
            tx,
        )
        .unwrap()
        .unwrap();

        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port as u16)).unwrap();
        write!(stream, "GET /health HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
        stream.flush().unwrap();
        let mut answer = String::new();
        let _ = stream.read_to_string(&mut answer);
        assert!(answer.contains("404"), "{answer}");
        assert!(rx.recv_timeout(Duration::from_millis(600)).is_err());
        armed.disarm();
    }

    /// A block that is not a source arms into nothing rather than an error.
    #[test]
    fn a_block_that_is_not_a_source_arms_into_nothing() {
        assert!(
            arm(&block("llm", &[]), Path::new("/tmp"), channel().0)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn a_home_path_expands_and_a_relative_one_lands_in_the_workspace() {
        unsafe { std::env::set_var("HOME", "/home/someone") };
        assert_eq!(
            expand("~/inbox", Path::new("/w")),
            PathBuf::from("/home/someone/inbox")
        );
        assert_eq!(expand("mail", Path::new("/w")), PathBuf::from("/w/mail"));
        assert_eq!(
            expand("/tmp/mail", Path::new("/w")),
            PathBuf::from("/tmp/mail")
        );
    }
}
