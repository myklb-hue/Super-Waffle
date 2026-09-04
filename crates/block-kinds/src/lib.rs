//! The built-in block catalogue: every kind's ports, category and settings.
//!
//! This is the definition the engine runs, the library panel lists and the
//! generated TypeScript describes, so there is exactly one place a port can be
//! added or retyped. It follows SPEC §6 row by row; where the specification and
//! a mockup disagreed, §6 won, and the difference is noted at the kind.

pub mod catalogue;
pub mod validate;

use graph_format::{PortType, Side};
use serde::{Deserialize, Serialize};
use specta::Type;

pub use catalogue::{KINDS, kind};
pub use validate::{Problem, validate};

/// The ten shelves of the library. A category carries its own colour, shared
/// between the library shelf and the block header (SPEC §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Models,
    Capabilities,
    Runtimes,
    Senses,
    Memory,
    Actuators,
    Data,
    Control,
    Human,
    Custom,
}

impl Category {
    pub const ALL: [Category; 10] = [
        Category::Models,
        Category::Capabilities,
        Category::Runtimes,
        Category::Senses,
        Category::Memory,
        Category::Actuators,
        Category::Data,
        Category::Control,
        Category::Human,
        Category::Custom,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Category::Models => "models",
            Category::Capabilities => "capabilities",
            Category::Runtimes => "runtimes",
            Category::Senses => "senses",
            Category::Memory => "memory",
            Category::Actuators => "actuators",
            Category::Data => "data",
            Category::Control => "control",
            Category::Human => "human",
            Category::Custom => "custom",
        }
    }

    /// The CSS token the colour resolves through. Never a hex: the value lives
    /// in `packages/ui/src/styles/tokens.css` and nowhere else.
    pub fn color_token(self) -> &'static str {
        match self {
            Category::Models => "cat-models",
            Category::Capabilities => "cat-capabilities",
            Category::Runtimes => "cat-runtimes",
            Category::Senses => "cat-senses",
            Category::Memory => "cat-memory",
            Category::Actuators => "cat-actuators",
            Category::Data => "cat-data",
            Category::Control => "cat-control",
            Category::Human => "cat-human",
            Category::Custom => "cat-custom",
        }
    }
}

/// One port a kind declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PortDef {
    pub name: &'static str,
    #[serde(rename = "type")]
    pub port_type: PortType,
    pub side: Side,
    /// An optional port is hidden until it is wired or the block is selected
    /// (SPEC §4.5).
    pub optional: bool,
    /// A dynamic port grows a new empty slot as each one is filled: the
    /// Toolbox's `tools` and the Memory hub's `memory` (SPEC §9).
    pub dynamic: bool,
}

pub(crate) const fn port(name: &'static str, port_type: PortType, side: Side) -> PortDef {
    PortDef {
        name,
        port_type,
        side,
        optional: false,
        dynamic: false,
    }
}

pub(crate) const fn optional(name: &'static str, port_type: PortType, side: Side) -> PortDef {
    PortDef {
        name,
        port_type,
        side,
        optional: true,
        dynamic: false,
    }
}

pub(crate) const fn dynamic(name: &'static str, port_type: PortType, side: Side) -> PortDef {
    PortDef {
        name,
        port_type,
        side,
        optional: false,
        dynamic: true,
    }
}

/// What kind of control the inspector draws for a setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SettingKind {
    Text,
    Multiline,
    Number,
    /// A number with a floor and a ceiling; the inspector draws a slider.
    Range,
    Bool,
    Select,
    /// A list of strings.
    List,
    /// A filesystem path.
    Path,
}

/// One setting a kind declares. Settings live in the inspector; ports live on
/// the block (SPEC §7).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SettingDef {
    pub name: &'static str,
    pub label: &'static str,
    pub kind: SettingKind,
    /// The options a `Select` offers.
    pub options: &'static [&'static str],
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// Why this setting matters, where that is not obvious. Shown as the hint
    /// under a switch row; this is where a safety or privacy boundary is stated
    /// in plain words (SPEC §12).
    pub hint: Option<&'static str>,
}

pub(crate) const fn setting(
    name: &'static str,
    label: &'static str,
    kind: SettingKind,
) -> SettingDef {
    SettingDef {
        name,
        label,
        kind,
        options: &[],
        min: None,
        max: None,
        hint: None,
    }
}

pub(crate) const fn select(
    name: &'static str,
    label: &'static str,
    options: &'static [&'static str],
) -> SettingDef {
    SettingDef {
        name,
        label,
        kind: SettingKind::Select,
        options,
        min: None,
        max: None,
        hint: None,
    }
}

pub(crate) const fn range(
    name: &'static str,
    label: &'static str,
    min: f64,
    max: f64,
) -> SettingDef {
    SettingDef {
        name,
        label,
        kind: SettingKind::Range,
        options: &[],
        min: Some(min),
        max: Some(max),
        hint: None,
    }
}

pub(crate) const fn hinted(mut def: SettingDef, hint: &'static str) -> SettingDef {
    def.hint = Some(hint);
    def
}

/// One entry in the catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BlockKind {
    /// The key a `.loom` file writes.
    pub id: &'static str,
    pub title: &'static str,
    pub category: Category,
    /// A name in the shared icon set.
    pub icon: &'static str,
    pub ports: &'static [PortDef],
    pub settings: &'static [SettingDef],
    /// A source emits on its own initiative and keeps the graph armed, so a
    /// graph that holds one never finishes (SPEC §8.2).
    pub source: bool,
    /// Whether this kind can be drawn in Stage view, which is to say whether it
    /// has a picture (SPEC §3.4).
    pub stage: bool,
    /// One line, shown in the library and the inspector.
    pub summary: &'static str,
}

impl BlockKind {
    pub fn port(&self, name: &str) -> Option<&PortDef> {
        self.ports.iter().find(|p| p.name == name)
    }

    pub fn inputs(&self) -> impl Iterator<Item = &PortDef> {
        self.ports.iter().filter(|p| p.side == Side::In)
    }

    pub fn outputs(&self) -> impl Iterator<Item = &PortDef> {
        self.ports.iter().filter(|p| p.side == Side::Out)
    }

    pub fn setting(&self, name: &str) -> Option<&SettingDef> {
        self.settings.iter().find(|s| s.name == name)
    }
}
