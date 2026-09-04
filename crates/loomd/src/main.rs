//! The engine, as a process.
//!
//!     loomd --workspace <path> [--socket <path>]
//!
//! With no `--socket` it speaks the protocol on stdin and stdout, which is how
//! the Tauri host starts it as a child and how a person can drive it by hand.
//! With one, it listens on a Unix socket, which is what Deploy-as-a-service
//! will use (SPEC §15.1).
use loomd::{Engine, Workspace};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut workspace: Option<PathBuf> = None;
    let mut socket: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--workspace" | "-w" => workspace = args.next().map(PathBuf::from),
            "--socket" | "-s" => socket = args.next().map(PathBuf::from),
            "--version" => {
                println!("loomd {}", loomd::VERSION);
                return ExitCode::SUCCESS;
            }
            "--help" | "-h" => {
                println!("usage: loomd --workspace <path> [--socket <path>]");
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("loomd: unknown argument `{other}`");
                return ExitCode::from(2);
            }
        }
    }

    let Some(root) = workspace else {
        eprintln!("loomd: --workspace is required");
        return ExitCode::from(2);
    };

    let engine = match Workspace::open(&root) {
        Ok(ws) => Engine::new(ws),
        Err(e) => {
            eprintln!("loomd: {e}");
            return ExitCode::FAILURE;
        }
    };

    match socket {
        None => {
            // `stdout()` rather than a lock: the writer runs on its own
            // thread, and a lock guard cannot cross one.
            engine.serve(std::io::stdin().lock(), std::io::stdout());
        }
        Some(path) => {
            // A stale socket from a crashed engine would otherwise make every
            // later start fail with "address in use".
            let _ = std::fs::remove_file(&path);
            let listener = match UnixListener::bind(&path) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("loomd: binding {}: {e}", path.display());
                    return ExitCode::FAILURE;
                }
            };
            eprintln!("loomd {} listening on {}", loomd::VERSION, path.display());
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let write = match stream.try_clone() {
                            Ok(w) => w,
                            Err(e) => {
                                eprintln!("loomd: {e}");
                                continue;
                            }
                        };
                        engine.serve(stream, write);
                    }
                    Err(e) => eprintln!("loomd: accept: {e}"),
                }
            }
        }
    }
    ExitCode::SUCCESS
}
