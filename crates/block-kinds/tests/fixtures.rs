//! The catalogue and the fixtures have to agree. If a fixture wires a port
//! that no kind declares, one of the two is wrong, and this is where that shows
//! up rather than at run time.

use block_kinds::{Category, KINDS, Problem, kind, validate};
use graph_format::{Endpoint, PortType, Side, Wire, from_str};
use std::fs;
use std::path::PathBuf;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/graphs")
}

fn each_fixture(mut f: impl FnMut(&str, graph_format::Graph)) {
    let mut paths: Vec<_> = fs::read_dir(fixtures())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "loom"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty());
    for path in paths {
        let text = fs::read_to_string(&path).unwrap();
        let name = path.file_name().unwrap().to_str().unwrap().to_owned();
        f(
            &name,
            from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}")),
        );
    }
}

#[test]
fn every_fixture_validates_clean() {
    each_fixture(|name, graph| {
        let problems = validate(&graph);
        assert!(
            problems.is_empty(),
            "{name} has problems:\n{}",
            problems
                .iter()
                .map(|p| format!("  - {p}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    });
}

/// Every kind a fixture uses is one the catalogue knows, which is what keeps
/// the examples in SPEC §13 honest.
#[test]
fn fixtures_only_use_catalogued_kinds() {
    each_fixture(|name, graph| {
        for b in &graph.blocks {
            assert!(
                b.kind == "custom" || kind(&b.kind).is_some(),
                "{name}: block {} uses unknown kind `{}`",
                b.id,
                b.kind
            );
        }
    });
}

/// Between them the four fixtures should exercise the whole grammar, or they
/// are not doing their job as the things every later slice is tested against.
#[test]
fn the_fixtures_cover_the_grammar() {
    let mut categories = Vec::new();
    let mut types = Vec::new();
    let mut has_frame = false;
    let mut has_custom_source = false;
    let mut has_dynamic_fan_in = false;

    each_fixture(|_, graph| {
        has_frame |= !graph.frames.is_empty();
        for b in &graph.blocks {
            if b.source.is_some() {
                has_custom_source = true;
            }
            if let Some(k) = kind(&b.kind)
                && !categories.contains(&k.category)
            {
                categories.push(k.category);
            }
        }
        for w in &graph.wires {
            if graph.wires.iter().filter(|o| o.to == w.to).count() > 1 {
                has_dynamic_fan_in = true;
            }
            if let Some(b) = graph.block(&w.from.node)
                && let Some(k) = kind(&b.kind)
                && let Some(p) = k
                    .ports
                    .iter()
                    .find(|p| p.name == w.from.port && p.side == Side::Out)
                && !types.contains(&p.port_type)
            {
                types.push(p.port_type);
            }
        }
    });

    assert!(has_frame, "no fixture uses a loop frame");
    assert!(has_custom_source, "no fixture uses a custom block");
    assert!(
        has_dynamic_fan_in,
        "no fixture fans two wires into one port"
    );

    for c in [
        Category::Models,
        Category::Capabilities,
        Category::Runtimes,
        Category::Senses,
        Category::Memory,
        Category::Actuators,
        Category::Data,
        Category::Control,
        Category::Human,
    ] {
        assert!(
            categories.contains(&c),
            "no fixture uses a {} block",
            c.as_str()
        );
    }

    for t in [
        PortType::Text,
        PortType::Tools,
        PortType::Memory,
        PortType::Image,
        PortType::Audio,
    ] {
        assert!(types.contains(&t), "no fixture carries a {t} wire");
    }
}

/// The check the whole type grammar exists to make: a wire that does not match
/// is refused, and the message says which pair failed.
#[test]
fn an_incompatible_wire_is_reported() {
    let text = fs::read_to_string(fixtures().join("home-assistant.loom")).unwrap();
    let mut graph = from_str(&text).unwrap();
    // The microphone emits audio; the orchestrator's prompt takes text.
    graph.wires.push(Wire {
        id: "bad".into(),
        from: Endpoint::new("microphone", "audio"),
        to: Endpoint::new("orchestrator", "prompt"),
    });
    let problems = validate(&graph);
    assert!(
        problems.contains(&Problem::Incompatible {
            wire: "bad".into(),
            from: PortType::Audio,
            to: PortType::Text,
        }),
        "expected an incompatibility, got {problems:?}"
    );
}

/// A handle is not a value, so it never reaches an `any` port even though the
/// port accepts "everything" (SPEC §4.1 versus §4.3).
#[test]
fn a_handle_cannot_land_on_an_any_port() {
    let text = fs::read_to_string(fixtures().join("customer-triage.loom")).unwrap();
    let mut graph = from_str(&text).unwrap();
    graph.wires.push(Wire {
        id: "bad".into(),
        from: Endpoint::new("terminal", "tool"),
        to: Endpoint::new("report", "value"),
    });
    let problems = validate(&graph);
    assert!(
        problems.contains(&Problem::Incompatible {
            wire: "bad".into(),
            from: PortType::Tools,
            to: PortType::Any,
        }),
        "tools should not reach an any port, got {problems:?}"
    );
}

/// Wiring backwards is its own message, because "no such port" would send the
/// user looking for a typo that is not there.
#[test]
fn a_backwards_wire_says_so() {
    let text = fs::read_to_string(fixtures().join("customer-triage.loom")).unwrap();
    let mut graph = from_str(&text).unwrap();
    graph.wires.push(Wire {
        id: "bad".into(),
        from: Endpoint::new("llm", "prompt"),
        to: Endpoint::new("report", "value"),
    });
    let problems = validate(&graph);
    assert!(
        problems.iter().any(|p| matches!(
            p,
            Problem::WrongDirection { wire, port, .. } if wire == "bad" && port == "prompt"
        )),
        "expected a direction problem, got {problems:?}"
    );
}

/// A block from a newer version keeps its wires, so installing the missing
/// kind restores the graph rather than leaving a hole (SPEC §12.1: warn, never
/// block).
#[test]
fn an_unknown_kind_is_a_warning_not_a_refusal() {
    let text = fs::read_to_string(fixtures().join("customer-triage.loom"))
        .unwrap()
        .replacen("kind: terminal", "kind: quantum-terminal", 1);
    let graph = from_str(&text).unwrap();
    let problems = validate(&graph);
    assert!(
        problems
            .iter()
            .any(|p| matches!(p, Problem::UnknownKind { .. }))
    );
    // The wires are still there.
    assert_eq!(graph.wires.len(), 5);
}

/// The catalogue is the size the specification describes. A count is a blunt
/// test, but it catches a kind quietly dropped in a refactor.
#[test]
fn the_catalogue_is_complete() {
    let built_in = KINDS
        .iter()
        .filter(|k| k.category != Category::Custom)
        .count();
    assert_eq!(built_in, 49, "the built-in catalogue changed size");
    assert_eq!(KINDS.len(), 50, "including the custom shelf itself");
}
