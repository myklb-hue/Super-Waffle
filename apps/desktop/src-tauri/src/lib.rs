//! The desktop host: a window, and a connection to the engine.
//!
//! The host is deliberately thin. It owns the window and the engine process and
//! nothing else — no graph logic, no validation, no format knowledge — because
//! everything it might do here would have to be done again in the headless
//! service (SPEC §15.1). Its whole job is to carry a request to `loomd` and a
//! reply back.
//!
//! In this slice the engine runs in-process, as a library, while the socket
//! client is built. That is a deliberate step, not the destination: the crate
//! boundary is already there, so moving to a child process changes this file
//! and nothing above it.

use loomd::session::{Outgoing, Session};
use loomd::{Engine, Reply, Request, RpcError, Workspace};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use tauri::Emitter;

/// The name the window listens on. One channel for every event, because the
/// shell already has to tell them apart by their tag.
const EVENTS: &str = "loomd:event";

struct Host {
    engine: Mutex<Option<Engine>>,
    /// Where a run sends what it has to say. The engine's own protocol puts
    /// replies and events down one queue; here the reply is the command's
    /// return value, so only events travel this way.
    session: Arc<Session>,
    /// Why the engine is not there, if it is not. Held so the window can say so
    /// rather than showing an empty canvas.
    failure: Mutex<Option<String>>,
}

/// One request, one reply. Errors come back as `Reply::Error` rather than as a
/// rejected promise, because an engine problem is something the shell shows,
/// not something it crashes on (SPEC §12.1).
#[tauri::command]
fn rpc(state: tauri::State<'_, Host>, request: Request) -> Reply {
    let engine = state.engine.lock().expect("the host's engine lock");
    match engine.as_ref() {
        Some(engine) => engine.handle(request, &state.session),
        None => {
            let why = state
                .failure
                .lock()
                .expect("the host's failure lock")
                .clone()
                .unwrap_or_else(|| "the engine did not start".to_owned());
            Reply::Error(RpcError::new("engine", why))
        }
    }
}

/// Which folder to serve.
///
/// `CYBERLOOM_WORKSPACE` first, so a test or a second window can point
/// somewhere else. Then the current directory *if it looks like a workspace*,
/// which is what a developer running from a checkout means. Otherwise
/// `~/Cyberloom`, made on the spot.
///
/// The middle case is the one that matters for a packaged build. An AppImage
/// launched from a desktop menu has a working directory of whatever the
/// launcher felt like — often `/` or the home folder — and serving that would
/// mean a first run that either shows nothing or offers to save graphs into
/// somebody's home directory. A folder that contains no graphs is not a
/// workspace someone meant to open.
fn workspace_root() -> PathBuf {
    if let Some(named) = std::env::var_os("CYBERLOOM_WORKSPACE") {
        return PathBuf::from(named);
    }
    if let Ok(here) = std::env::current_dir()
        && looks_like_a_workspace(&here)
    {
        return here;
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let mine = home.join("Cyberloom");
    // Made rather than reported: a first run should open a window with a
    // canvas in it, not an error about a folder that has never existed.
    let _ = std::fs::create_dir_all(mine.join("graphs"));
    mine
}

/// Whether a folder is somebody's workspace rather than wherever the launcher
/// happened to start.
fn looks_like_a_workspace(folder: &std::path::Path) -> bool {
    if folder.join("workspace.yaml").is_file() {
        return true;
    }
    let graphs = folder.join("graphs");
    std::fs::read_dir(if graphs.is_dir() { &graphs } else { folder })
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .any(|e| e.path().extension().is_some_and(|x| x == "loom"))
        })
        .unwrap_or(false)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let root = workspace_root();
    let (engine, failure) = match Workspace::open(&root) {
        Ok(ws) => (Some(Engine::new(ws)), None),
        Err(e) => (None, Some(format!("{} ({e})", root.display()))),
    };

    let (tx, rx) = channel::<Outgoing>();
    let session = Arc::new(Session::new(tx));

    tauri::Builder::default()
        .manage(Host {
            engine: Mutex::new(engine),
            session,
            failure: Mutex::new(failure),
        })
        .setup(move |app| {
            // One thread turning the engine's queue into window events. It is
            // the same shape as the socket's writer thread and for the same
            // reason: a run says things while the shell is asking other
            // questions, and neither may wait on the other.
            let handle = app.handle().clone();
            std::thread::Builder::new()
                .name("cyberloom-events".into())
                .spawn(move || {
                    for message in rx {
                        let Outgoing::Line(line) = message else { break };
                        match serde_json::from_str::<serde_json::Value>(&line) {
                            Ok(event) => {
                                let _ = handle.emit(EVENTS, event);
                            }
                            // The engine only ever queues what it serialised
                            // itself, so this cannot happen; dropping it beats
                            // taking the window down if it ever does.
                            Err(_) => continue,
                        }
                    }
                })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![rpc])
        .run(tauri::generate_context!())
        .expect("the window could not be created");
}
