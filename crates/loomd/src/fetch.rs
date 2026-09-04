//! Downloading weights, explicitly and resumably (SPEC §15.13, slice 12).
//!
//! Two rules, and everything here follows from them. **Explicit**: nothing is
//! fetched because a graph happened to run. A model arrives because a person
//! asked for it, which is why this is a request the shell makes and not
//! something the perception provider does behind a block. **Resumable**: a
//! weights file is hundreds of megabytes on a connection that may not last that
//! long, and a download that starts from zero every time is a download that
//! never finishes on a bad line.
//!
//! Offline is a supported state, not an error condition: a workspace with no
//! weights runs every graph that does not need them, and the ones that do say
//! what is missing on the block that needs it.

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// How much arrives before the caller hears about it again.
const REPORT_EVERY: u64 = 512 * 1024;

/// What a download is doing, for the shell's progress line.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    pub had: u64,
    pub got: u64,
    /// The total, when the server said. A server that will not say is not a
    /// failure; it is a download without a percentage.
    pub total: Option<u64>,
}

impl Progress {
    pub fn fraction(&self) -> Option<f64> {
        let total = self.total?;
        (total > 0).then(|| (self.got as f64 / total as f64).clamp(0.0, 1.0))
    }
}

/// Fetch `url` into `into`, continuing from whatever is already there.
///
/// The partial file is `<into>.part`, and it is what makes this resumable: its
/// length is the offset asked for with `Range`, and the finished file only
/// appears when the whole thing has arrived. A reader that finds `<into>` finds
/// something complete, always — there is no window in which a half-written file
/// looks like a model.
pub fn resumable(url: &str, into: &Path, say: &mut dyn FnMut(Progress)) -> Result<PathBuf, String> {
    if into.exists() {
        return Ok(into.to_path_buf());
    }
    if let Some(parent) = into.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("could not make {}: {e}", parent.display()))?;
    }
    let part = into.with_extension(format!(
        "{}part",
        into.extension()
            .map(|e| format!("{}.", e.to_string_lossy()))
            .unwrap_or_default()
    ));
    let had = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);

    let mut request = ureq::get(url);
    if had > 0 {
        request = request.header("range", &format!("bytes={had}-"));
    }
    let mut answer = request
        .call()
        .map_err(|e| format!("could not reach {url}: {e}"))?;

    let status = answer.status().as_u16();
    // 206 means the server honoured the range and is sending the rest. 200
    // means it is sending the whole thing — either it does not do ranges or it
    // decided not to — so what is already on disk is worthless and the file
    // starts again. Appending to it would produce a corrupt file that looks
    // exactly like a good one.
    let resuming = status == 206;
    let total = answer
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .map(|len| if resuming { len + had } else { len });

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(!resuming)
        .open(&part)
        .map_err(|e| format!("could not open {}: {e}", part.display()))?;
    if resuming {
        file.seek(SeekFrom::End(0))
            .map_err(|e| format!("could not continue {}: {e}", part.display()))?;
    }

    let mut got = if resuming { had } else { 0 };
    let mut since = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    let mut body = answer.body_mut().as_reader();
    say(Progress { had, got, total });
    // A connection that dies mid-download is the ordinary case this function
    // exists for, not an exception: what has arrived is kept, and the error
    // says to ask again. Returning here would throw away the bytes that make
    // the next attempt short.
    let mut cut_off = None;
    loop {
        let read = match body.read(&mut buffer) {
            Ok(read) => read,
            Err(e) => {
                cut_off = Some(format!("the connection stopped: {e}"));
                break;
            }
        };
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|e| format!("could not write {}: {e}", part.display()))?;
        got += read as u64;
        since += read as u64;
        if since >= REPORT_EVERY {
            since = 0;
            say(Progress { had, got, total });
        }
    }
    file.flush()
        .map_err(|e| format!("could not write {}: {e}", part.display()))?;
    drop(file);

    // A server that promised a length and sent less has been cut off. The part
    // file stays, so the next attempt continues rather than starting over —
    // which is the entire point of doing it this way.
    let short = total.is_some_and(|total| got < total);
    if cut_off.is_some() || short {
        say(Progress { had, got, total });
        return Err(match total {
            Some(total) => {
                format!("{url} stopped after {got} of {total} bytes. Ask again to continue.")
            }
            None => format!("{url} stopped after {got} bytes. Ask again to continue."),
        });
    }

    std::fs::rename(&part, into)
        .map_err(|e| format!("could not finish {}: {e}", into.display()))?;
    say(Progress {
        had,
        got,
        total: total.or(Some(got)),
    });
    Ok(into.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufRead;
    use std::net::TcpListener;

    /// A server that speaks enough HTTP to be downloaded from, and can be told
    /// to hang up part way.
    ///
    /// A real server rather than a mocked client: what is being tested is the
    /// `Range` handshake and what happens to the bytes on disk, and both of
    /// those live between two processes.
    fn serving(body: Vec<u8>, cut_after: Option<usize>) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let thread = std::thread::spawn(move || {
            for stream in listener.incoming().take(1) {
                let Ok(mut stream) = stream else { continue };
                let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
                let mut from = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        break;
                    }
                    if let Some(range) = line.to_lowercase().strip_prefix("range: bytes=") {
                        from = range
                            .split('-')
                            .next()
                            .and_then(|n| n.trim().parse().ok())
                            .unwrap_or(0);
                    }
                    if line.trim().is_empty() {
                        break;
                    }
                }
                let rest = &body[from.min(body.len())..];
                let send = match cut_after {
                    Some(n) => &rest[..n.min(rest.len())],
                    None => rest,
                };
                let head = if from > 0 {
                    format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\n\
                         Content-Range: bytes {from}-{}/{}\r\n\r\n",
                        rest.len(),
                        body.len() - 1,
                        body.len()
                    )
                } else {
                    format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", rest.len())
                };
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(send);
                let _ = stream.flush();
            }
        });
        (format!("http://127.0.0.1:{port}/weights.onnx"), thread)
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cyberloom-fetch-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_download_arrives_whole() {
        let dir = scratch("whole");
        let body: Vec<u8> = (0..4096u32).map(|n| n as u8).collect();
        let (url, server) = serving(body.clone(), None);
        let mut seen = Vec::new();
        let path = resumable(&url, &dir.join("weights.onnx"), &mut |p| seen.push(p)).unwrap();
        server.join().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), body);
        assert_eq!(seen.last().unwrap().got, body.len() as u64);
        assert_eq!(seen.last().unwrap().fraction(), Some(1.0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The whole point: a connection that dies leaves something to continue
    /// from, and the second attempt asks for the rest rather than for all of it.
    #[test]
    fn a_download_that_is_cut_off_continues_where_it_stopped() {
        let dir = scratch("resume");
        let body: Vec<u8> = (0..8192u32).map(|n| (n % 251) as u8).collect();
        let into = dir.join("weights.onnx");

        let (url, server) = serving(body.clone(), Some(3000));
        let stopped = resumable(&url, &into, &mut |_| {}).unwrap_err();
        server.join().unwrap();
        assert!(stopped.contains("Ask again to continue"), "{stopped}");
        assert!(!into.exists(), "a half file must not look like a model");
        let part = dir.join("weights.onnx.part");
        assert_eq!(std::fs::metadata(&part).unwrap().len(), 3000);

        let (url, server) = serving(body.clone(), None);
        let mut seen = Vec::new();
        let path = resumable(&url, &into, &mut |p| seen.push(p)).unwrap();
        server.join().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), body);
        assert_eq!(seen[0].had, 3000, "it should have continued: {seen:?}");
        assert!(!part.exists(), "the part file should be gone");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Nothing is fetched twice. A model that is there is a model that is
    /// there, and a graph that runs every minute must not re-check the network.
    #[test]
    fn what_is_already_downloaded_is_not_downloaded_again() {
        let dir = scratch("already");
        let into = dir.join("weights.onnx");
        std::fs::write(&into, b"here already").unwrap();
        // No server at all: reaching the network would fail, and it must not
        // even try.
        let path = resumable("http://127.0.0.1:1/nothing", &into, &mut |_| {}).unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"here already");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_download_with_no_length_is_a_download_without_a_percentage() {
        let progress = Progress {
            had: 0,
            got: 10,
            total: None,
        };
        assert_eq!(progress.fraction(), None);
    }
}
