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

use loomd::{Engine, Reply, Request, RpcError, Workspace};
use std::path::PathBuf;
use std::sync::Mutex;

struct Host {
    engine: Mutex<Option<Engine>>,
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
        Some(engine) => engine.handle(request),
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
/// somewhere else; then the current directory, which is what a developer
/// running from a checkout means.
fn workspace_root() -> PathBuf {
    std::env::var_os("CYBERLOOM_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let root = workspace_root();
    let (engine, failure) = match Workspace::open(&root) {
        Ok(ws) => (Some(Engine::new(ws)), None),
        Err(e) => (None, Some(format!("{} ({e})", root.display()))),
    };

    tauri::Builder::default()
        .manage(Host {
            engine: Mutex::new(engine),
            failure: Mutex::new(failure),
        })
        .invoke_handler(tauri::generate_handler![rpc])
        .run(tauri::generate_context!())
        .expect("the window could not be created");
}
