//! The type grammar: port types and the one rule that decides whether a wire
//! is legal (SPEC §4.1).

use serde::{Deserialize, Serialize};
use specta::Type;
use std::fmt;
use std::str::FromStr;

/// The ten port types. A wire is coloured by its type, and the type is what
/// makes a connection legal or refuses it mid-drag. There is no untyped wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum PortType {
    /// Prompts, stdout, any string.
    Text,
    /// One callable, or a bundle of them. A handle, not a flow.
    Tools,
    /// A store the model reads and writes. A handle, not a flow.
    Memory,
    /// Structured json or a record.
    Data,
    /// Output arriving incrementally.
    Stream,
    /// Frames from a camera or a file.
    Image,
    /// Samples from a microphone.
    Audio,
    /// A path or blob on disk.
    File,
    /// A trigger or control flow, never a value.
    Exec,
    /// Accepts every value type.
    Any,
}

impl PortType {
    pub const ALL: [PortType; 10] = [
        PortType::Text,
        PortType::Tools,
        PortType::Memory,
        PortType::Data,
        PortType::Stream,
        PortType::Image,
        PortType::Audio,
        PortType::File,
        PortType::Exec,
        PortType::Any,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            PortType::Text => "text",
            PortType::Tools => "tools",
            PortType::Memory => "memory",
            PortType::Data => "data",
            PortType::Stream => "stream",
            PortType::Image => "image",
            PortType::Audio => "audio",
            PortType::File => "file",
            PortType::Exec => "exec",
            PortType::Any => "any",
        }
    }

    /// A closed type carries something that is not a value: a handle the holder
    /// calls (`tools`, `memory`) or control flow (`exec`). Closed types connect
    /// only to a port of the same type — in particular an `any` port does *not*
    /// accept them, because `any` means "any value" and these are not values.
    ///
    /// SPEC §4.1 states this for each type in its own row (`exec` is accepted
    /// by `exec`; `tools` by `llm.tools` and Toolbox inputs; `memory` by
    /// `llm.memory` and hub inputs) while the `any` row reads "everything".
    /// This method is where the two are reconciled, and the narrower reading
    /// wins because §4.3 makes handles and flows different things.
    pub fn is_closed(self) -> bool {
        matches!(self, PortType::Tools | PortType::Memory | PortType::Exec)
    }

    /// A handle is something the holder *calls*: `tools` and `memory`
    /// (SPEC §4.3). Every handle is closed, but not every closed type is a
    /// handle — `exec` is control flow, and control flow is neither a value
    /// nor something to call.
    ///
    /// The distinction decides what runs. A block whose only wired outputs are
    /// handles is a capability, waiting to be called; a block whose outputs are
    /// `exec` is a step that sets other steps going, and treating it as a
    /// capability would mean a Branch never ran at all.
    pub fn is_handle(self) -> bool {
        matches!(self, PortType::Tools | PortType::Memory)
    }

    /// Whether a wire from a `self` output may land on a `target` input.
    ///
    /// The whole rule, with no implicit cast. Where two types are compatible
    /// but not identical the shell offers to insert a Convert block on the
    /// wire; the conversion is always visible (SPEC §5.4, §15.5).
    pub fn accepted_by(self, target: PortType) -> bool {
        use PortType::*;
        match (self, target) {
            // A closed type only ever meets its own kind, in either direction.
            (a, b) if a.is_closed() || b.is_closed() => a == b,
            // `any` is the universal value type, at either end.
            (Any, _) | (_, Any) => true,
            // A stream of text is text; a stream of records is data.
            (Stream, Text) | (Stream, Data) => true,
            (a, b) => a == b,
        }
    }
}

impl fmt::Display for PortType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PortType {
    type Err = UnknownPortType;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        PortType::ALL
            .into_iter()
            .find(|t| t.as_str() == s)
            .ok_or_else(|| UnknownPortType(s.to_owned()))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown port type `{0}`")]
pub struct UnknownPortType(pub String);

/// Which side of a block a port sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    In,
    Out,
}

/// How much of a block is drawn. Every block has compact and summary; a custom
/// block's third view is its code, a block with a picture is its stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum View {
    Compact,
    #[default]
    Summary,
    Code,
    Stage,
}

impl View {
    pub fn as_str(self) -> &'static str {
        match self {
            View::Compact => "compact",
            View::Summary => "summary",
            View::Code => "code",
            View::Stage => "stage",
        }
    }
}

/// How a graph runs. Paused is a run state, not a graph setting, so it is not
/// here (SPEC §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum RunMode {
    #[default]
    Once,
    Live,
    Schedule,
}

impl RunMode {
    pub fn as_str(self) -> &'static str {
        match self {
            RunMode::Once => "once",
            RunMode::Live => "live",
            RunMode::Schedule => "schedule",
        }
    }
}

/// What happens when an event arrives while the previous one is still running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum OverlapPolicy {
    #[default]
    Queue,
    DropNewest,
    DropOldest,
    Coalesce,
}

impl OverlapPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            OverlapPolicy::Queue => "queue",
            OverlapPolicy::DropNewest => "dropNewest",
            OverlapPolicy::DropOldest => "dropOldest",
            OverlapPolicy::Coalesce => "coalesce",
        }
    }
}

/// Where a custom block's code lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum SourceMode {
    #[default]
    Inline,
    File,
}

impl SourceMode {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceMode::Inline => "inline",
            SourceMode::File => "file",
        }
    }
}

/// The languages a custom block may be written in (SPEC §15.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Python,
    /// The TypeScript kind runs plain JavaScript too.
    Typescript,
    Javascript,
    Shell,
}

impl Language {
    pub fn as_str(self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::Typescript => "typescript",
            Language::Javascript => "javascript",
            Language::Shell => "shell",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PortType::*;
    use super::*;

    /// The acceptance table in SPEC §4.1, row by row.
    #[test]
    fn spec_4_1_acceptance_table() {
        // text is accepted by text, any
        assert!(Text.accepted_by(Text));
        assert!(Text.accepted_by(Any));
        assert!(!Text.accepted_by(Data));

        // data is accepted by data, any — notably NOT text
        assert!(Data.accepted_by(Data));
        assert!(Data.accepted_by(Any));
        assert!(!Data.accepted_by(Text));

        // stream is accepted by text and data
        assert!(Stream.accepted_by(Text));
        assert!(Stream.accepted_by(Data));
        assert!(Stream.accepted_by(Any));
        assert!(!Stream.accepted_by(Image));

        // image, audio and file are accepted by their own type and any
        for t in [Image, Audio, File] {
            assert!(t.accepted_by(t));
            assert!(t.accepted_by(Any));
            assert!(!t.accepted_by(Text));
        }

        // any is accepted everywhere a value is accepted
        for t in [Text, Data, Stream, Image, Audio, File, Any] {
            assert!(Any.accepted_by(t), "any should reach {t}");
        }
    }

    /// Handles and control flow are not values, so they never meet an `any`
    /// port in either direction.
    #[test]
    fn closed_types_only_meet_their_own_kind() {
        for closed in [Tools, Memory, Exec] {
            assert!(closed.accepted_by(closed));
            assert!(!closed.accepted_by(Any), "{closed} should not reach any");
            assert!(!Any.accepted_by(closed), "any should not reach {closed}");
            for other in [Text, Data, Stream, Image, Audio, File] {
                assert!(!closed.accepted_by(other));
                assert!(!other.accepted_by(closed));
            }
        }
        assert!(!Tools.accepted_by(Memory));
        assert!(!Exec.accepted_by(Tools));
    }

    #[test]
    fn every_type_accepts_itself() {
        for t in PortType::ALL {
            assert!(t.accepted_by(t), "{t} should accept itself");
        }
    }

    #[test]
    fn names_round_trip() {
        for t in PortType::ALL {
            assert_eq!(t.as_str().parse::<PortType>().unwrap(), t);
        }
    }
}
