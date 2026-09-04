//! The parser against the graph that already holds a custom block.
//!
//! `door-watch.loom` was written by hand in slice 1, before anything could
//! parse a signature: its `ports` list was typed out beside its code. That
//! makes it exactly the right test — if the parser and the hand-written
//! interface disagree, one of them is wrong, and until now nothing could tell.

use block_source::{Control, parse};
use graph_format::{Language, Side};

#[test]
fn the_parser_agrees_with_the_hand_written_fixture() {
    let graph = graph_format::load(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/graphs/door-watch.loom"
    ))
    .unwrap();

    let block = graph
        .blocks
        .iter()
        .find(|b| b.kind == "custom")
        .expect("door-watch holds a custom block");
    let source = block.source.as_ref().expect("it has inline code");
    let code = source.code.as_deref().expect("the code is in the file");

    let derived = parse(source.language, code).unwrap();
    assert_eq!(derived.len(), 1);
    let interface = &derived[0];

    // The title the file gives the block is the function's own name.
    assert_eq!(block.title.as_deref(), Some(interface.name.as_str()));
    assert_eq!(
        interface.description.as_deref(),
        Some("Is the front door open?")
    );

    // Every port the hand-written file declares, the code produces — except
    // `memory`, which the fixture added ahead of SPEC §13.4's later version of
    // the function. That is the one difference, and it is the fixture that is
    // ahead of its own code rather than the parser that is behind.
    let derived_ports: Vec<(&str, Side)> = interface
        .ports
        .iter()
        .map(|p| (p.name.as_str(), p.side))
        .collect();
    assert_eq!(derived_ports, [("frame", Side::In), ("result", Side::Out)]);

    for name in ["frame", "result"] {
        let written = block.ports.iter().find(|p| p.name == name).unwrap();
        let read = interface.ports.iter().find(|p| p.name == name).unwrap();
        assert_eq!(
            written.port_type, read.port_type,
            "{name} is typed the same"
        );
        assert_eq!(written.side, read.side, "{name} is on the same side");
    }

    // And the threshold the file stores as a setting is the one the code
    // declares, with the slider SPEC §13.4 describes.
    assert_eq!(interface.settings.len(), 1);
    assert_eq!(interface.settings[0].name, "threshold");
    assert_eq!(interface.settings[0].kind, Control::Range);
    assert_eq!(interface.settings[0].default, "0.6");
    assert_eq!(
        block.settings.get("threshold"),
        Some(&graph_format::Setting::Float(0.6))
    );
}

/// SPEC §13.4's grown-up version: 184 lines and a `Memory` parameter. The
/// block gains a memory port and nothing else changes.
#[test]
fn adding_a_memory_parameter_adds_a_port_and_leaves_the_rest() {
    let before = parse(
        Language::Python,
        "def door_check(frame: Image, threshold: float = 0.6) -> Data:\n    pass\n",
    )
    .unwrap();
    let after = parse(
        Language::Python,
        "def door_check(frame: Image, memory: Memory, threshold: float = 0.6) -> Data:\n    pass\n",
    )
    .unwrap();

    let names = |i: &block_source::Interface| -> Vec<String> {
        i.ports.iter().map(|p| p.name.clone()).collect()
    };
    assert_eq!(names(&before[0]), ["frame", "result"]);
    assert_eq!(names(&after[0]), ["frame", "memory", "result"]);
    // The settings are untouched, so the threshold the user set survives.
    assert_eq!(before[0].settings, after[0].settings);
}

/// Every language spells the same block, and they produce the same interface.
/// That is what makes the language a detail of the file rather than a
/// difference in what a block is.
#[test]
fn the_three_languages_describe_the_same_block() {
    let python = parse(
        Language::Python,
        "def door_check(frame: Image, threshold: float = 0.6) -> Data:\n    pass\n",
    )
    .unwrap();
    let typescript = parse(
        Language::Typescript,
        "export function door_check(frame: Image, threshold: number = 0.6): Data {\n}\n",
    )
    .unwrap();
    let shell = parse(
        Language::Shell,
        "# @block door_check\n# @in frame: Image\n# @out result: Data\n# @set threshold: float = 0.6\n",
    )
    .unwrap();

    for other in [&typescript[0], &shell[0]] {
        assert_eq!(other.name, python[0].name);
        assert_eq!(other.ports, python[0].ports);
        assert_eq!(other.settings[0].name, python[0].settings[0].name);
        assert_eq!(other.settings[0].kind, python[0].settings[0].kind);
    }
}
