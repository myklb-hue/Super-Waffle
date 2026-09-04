//! A workspace is a folder (SPEC §15.6).
//!
//! Everything the engine serves is relative to one root, so a path that crosses
//! the conversation is the same path on any machine and a graph that references
//! a sibling file keeps working when the folder moves.

use graph_format::Graph;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("{0} is not a folder")]
    NotAFolder(String),
    #[error("`{0}` leaves the workspace")]
    Escapes(String),
    #[error("{path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("{0}")]
    Parse(String),
}

pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, WorkspaceError> {
        let root = root.as_ref();
        if !root.is_dir() {
            return Err(WorkspaceError::NotAFolder(root.display().to_string()));
        }
        let root = root.canonicalize().map_err(|source| WorkspaceError::Io {
            path: root.display().to_string(),
            source,
        })?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Turn a relative path from a request into an absolute one, refusing
    /// anything that would climb out of the workspace.
    ///
    /// The engine listens on a socket, so a request is not necessarily the
    /// shell: `..` and absolute paths are refused here rather than trusted.
    /// This is not a permission gate — the user owns their machine (SPEC §12) —
    /// it is a statement about what a *workspace path* means.
    pub fn resolve(&self, relative: &str) -> Result<PathBuf, WorkspaceError> {
        let candidate = Path::new(relative);
        if candidate.is_absolute() {
            return Err(WorkspaceError::Escapes(relative.to_owned()));
        }
        let mut depth = 0i32;
        for part in candidate.components() {
            match part {
                Component::Normal(_) => depth += 1,
                Component::CurDir => {}
                Component::ParentDir => {
                    depth -= 1;
                    if depth < 0 {
                        return Err(WorkspaceError::Escapes(relative.to_owned()));
                    }
                }
                _ => return Err(WorkspaceError::Escapes(relative.to_owned())),
            }
        }
        Ok(self.root.join(candidate))
    }

    /// Every `.loom` file in the workspace, sorted, so two runs list them the
    /// same way.
    pub fn graphs(&self) -> Result<Vec<PathBuf>, WorkspaceError> {
        let mut out = Vec::new();
        collect(&self.root, &mut out)?;
        out.sort();
        Ok(out)
    }

    /// The path a graph is known by: relative to the root, forward slashes.
    pub fn relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }

    /// Write a graph in canonical form.
    ///
    /// Returns whether anything was actually written. A save that would produce
    /// the bytes already on disk does nothing, so autosaving on every keystroke
    /// does not churn the file's timestamp or wake a file watcher for nothing.
    ///
    /// The write goes to a temporary file in the same directory and is then
    /// renamed over the target, so a crash mid-write leaves the old graph
    /// intact rather than a truncated one.
    pub fn save(&self, relative: &str, graph: &Graph) -> Result<bool, WorkspaceError> {
        let path = self.resolve(relative)?;
        let text = graph_format::to_string(graph);
        if std::fs::read_to_string(&path).is_ok_and(|existing| existing == text) {
            return Ok(false);
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|source| WorkspaceError::Io {
                path: dir.display().to_string(),
                source,
            })?;
        }
        let temp = path.with_extension("loom.tmp");
        std::fs::write(&temp, &text).map_err(|source| WorkspaceError::Io {
            path: temp.display().to_string(),
            source,
        })?;
        std::fs::rename(&temp, &path).map_err(|source| WorkspaceError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Ok(true)
    }

    pub fn load(&self, relative: &str) -> Result<Graph, WorkspaceError> {
        let path = self.resolve(relative)?;
        graph_format::load(&path).map_err(|e| match e {
            graph_format::LoadError::Io { path, source } => WorkspaceError::Io { path, source },
            other => WorkspaceError::Parse(other.to_string()),
        })
    }
}

/// Walks the folder, skipping the places a graph is never kept: the engine's
/// own cache, and anything a package manager or git owns.
fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), WorkspaceError> {
    const SKIP: [&str; 5] = [".cyberloom", ".git", "node_modules", "target", ".venv"];
    let entries = std::fs::read_dir(dir).map_err(|source| WorkspaceError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| WorkspaceError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !SKIP.contains(&name.as_ref()) {
                collect(&path, out)?;
            }
        } else if path.extension().is_some_and(|e| e == "loom") {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> Workspace {
        Workspace::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures")).unwrap()
    }

    #[test]
    fn finds_every_graph_in_order() {
        let ws = fixtures();
        let names: Vec<_> = ws
            .graphs()
            .unwrap()
            .iter()
            .map(|p| ws.relative(p))
            .collect();
        assert_eq!(
            names,
            [
                "graphs/customer-triage.loom",
                "graphs/door-watch.loom",
                "graphs/home-assistant.loom",
                "graphs/inbox-triage.loom",
            ]
        );
    }

    #[test]
    fn opens_a_graph_by_its_relative_path() {
        let ws = fixtures();
        let graph = ws.load("graphs/customer-triage.loom").unwrap();
        assert_eq!(graph.id, "customer-triage");
    }

    #[test]
    fn refuses_a_path_that_leaves_the_workspace() {
        let ws = fixtures();
        for bad in ["../secrets.loom", "graphs/../../etc/passwd", "/etc/passwd"] {
            assert!(
                matches!(ws.resolve(bad), Err(WorkspaceError::Escapes(_))),
                "{bad} should not resolve"
            );
        }
        // Climbing and coming back is fine: it stays inside.
        assert!(ws.resolve("graphs/../graphs/customer-triage.loom").is_ok());
    }

    #[test]
    fn a_missing_graph_says_which_one() {
        let ws = fixtures();
        let err = ws.load("graphs/nope.loom").unwrap_err().to_string();
        assert!(err.contains("nope.loom"), "unhelpful error: {err}");
    }
}
