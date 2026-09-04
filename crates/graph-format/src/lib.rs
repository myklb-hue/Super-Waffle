//! The `.loom` graph file: one readable YAML document with a canonical form.
//!
//! A graph edited by hand and one edited on the canvas produce the same file
//! (SPEC §15.3), which is what makes these files reviewable in a pull request
//! and mergeable in git. That guarantee is the reason the emitter is written by
//! hand: key order, quoting, block scalars and number formatting are decisions
//! the format makes, not the ones a general YAML library would make for it.
//!
//! ```
//! use graph_format::{from_str, to_string};
//! # let text = include_str!("../../../fixtures/graphs/customer-triage.loom");
//! let graph = from_str(text).unwrap();
//! assert_eq!(to_string(&graph), text);   // canonical, byte for byte
//! ```

pub mod model;
pub mod read;
pub mod types;
pub mod write;

pub use model::*;
pub use read::{ReadError, from_str};
pub use types::*;
pub use write::to_string;

use std::io;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("{path}: {source}")]
    Io { path: String, source: io::Error },
    #[error("{path}: {source}")]
    Parse { path: String, source: ReadError },
}

/// Read a graph from disk.
pub fn load(path: impl AsRef<Path>) -> Result<Graph, LoadError> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path).map_err(|source| LoadError::Io {
        path: path.display().to_string(),
        source,
    })?;
    from_str(&text).map_err(|source| LoadError::Parse {
        path: path.display().to_string(),
        source,
    })
}

/// Write a graph to disk in canonical form.
pub fn save(path: impl AsRef<Path>, graph: &Graph) -> Result<(), LoadError> {
    let path = path.as_ref();
    std::fs::write(path, to_string(graph)).map_err(|source| LoadError::Io {
        path: path.display().to_string(),
        source,
    })
}
