//! The guarantee the format exists for: reading a `.loom` file and writing it
//! back produces the same bytes, so a graph is reviewable in a pull request and
//! mergeable in git (SPEC §15.3).

use graph_format::{Position, from_str, to_string};
use std::fs;
use std::path::PathBuf;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/graphs")
}

fn each_fixture(mut f: impl FnMut(&str, &str)) {
    let dir = fixtures();
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "loom"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no fixtures found in {}", dir.display());
    for path in paths {
        let text = fs::read_to_string(&path).unwrap();
        f(path.file_name().unwrap().to_str().unwrap(), &text);
    }
}

#[test]
fn every_fixture_round_trips_byte_identical() {
    each_fixture(|name, text| {
        let graph = from_str(text).unwrap_or_else(|e| panic!("{name}: {e}"));
        let written = to_string(&graph);
        assert_eq!(written, text, "{name} is not in canonical form");
    });
}

#[test]
fn every_fixture_survives_two_passes() {
    each_fixture(|name, text| {
        let once = to_string(&from_str(text).unwrap());
        let twice = to_string(&from_str(&once).unwrap());
        assert_eq!(once, twice, "{name} is not stable under a second pass");
    });
}

/// Ids have to be unique or a wire cannot say what it connects.
#[test]
fn fixture_ids_are_unique() {
    each_fixture(|name, text| {
        let g = from_str(text).unwrap();
        let mut ids: Vec<_> = g.blocks.iter().map(|b| b.id.as_str()).collect();
        let before = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), before, "{name} has duplicate block ids");

        let mut wire_ids: Vec<_> = g.wires.iter().map(|w| w.id.as_str()).collect();
        let before = wire_ids.len();
        wire_ids.sort_unstable();
        wire_ids.dedup();
        assert_eq!(wire_ids.len(), before, "{name} has duplicate wire ids");
    });
}

/// Every wire has to name nodes that exist, in both directions. A node is a
/// block or a loop frame; frames have ports too.
#[test]
fn fixture_wires_reach_real_nodes() {
    each_fixture(|name, text| {
        let g = from_str(text).unwrap();
        for w in &g.wires {
            assert!(
                g.has_node(&w.from.node),
                "{name}: wire {} leaves a node that does not exist: {}",
                w.id,
                w.from.node
            );
            assert!(
                g.has_node(&w.to.node),
                "{name}: wire {} arrives at a node that does not exist: {}",
                w.id,
                w.to.node
            );
        }
        for b in &g.blocks {
            if let Some(frame) = &b.frame {
                assert!(
                    g.frame(frame).is_some(),
                    "{name}: block {} names frame {frame}, which does not exist",
                    b.id
                );
            }
        }
    });
}

/// Positions in a committed fixture are already on the grid, so opening one and
/// saving it produces no diff.
#[test]
fn fixture_positions_are_on_the_grid() {
    each_fixture(|name, text| {
        let g = from_str(text).unwrap();
        for b in &g.blocks {
            assert_eq!(
                b.position,
                b.position.snapped(),
                "{name}: block {} is off the grid",
                b.id
            );
        }
        for f in &g.frames {
            assert_eq!(
                f.position,
                f.position.snapped(),
                "{name}: frame {} is off the grid",
                f.id
            );
        }
    });
}

/// An off-grid position is snapped on the way out, which is what stops a
/// one-pixel nudge from showing up as a diff.
#[test]
fn saving_snaps_a_nudged_block_to_the_grid() {
    let text = fs::read_to_string(fixtures().join("customer-triage.loom")).unwrap();
    let mut g = from_str(&text).unwrap();
    let p = g.blocks[0].position;
    g.blocks[0].position = Position {
        x: p.x + 3.0,
        y: p.y - 4.0,
    };
    assert_eq!(
        to_string(&g),
        text,
        "a nudge inside half a grid step should leave no diff"
    );
}

/// A file this build cannot read is refused by name, not guessed at.
#[test]
fn a_future_version_is_refused() {
    let text = fs::read_to_string(fixtures().join("customer-triage.loom"))
        .unwrap()
        .replacen("version: 1", "version: 2", 1);
    let err = from_str(&text).unwrap_err().to_string();
    assert!(
        err.contains("unsupported format version 2"),
        "unhelpful error: {err}"
    );
}

/// An error names the path that failed, because that is what makes a bad file
/// fixable by hand.
#[test]
fn errors_name_their_location() {
    let text = fs::read_to_string(fixtures().join("customer-triage.loom"))
        .unwrap()
        .replacen("    position: [286, 154]\n", "", 1);
    let err = from_str(&text).unwrap_err().to_string();
    assert!(err.contains("blocks[0].position"), "unhelpful error: {err}");

    let text = fs::read_to_string(fixtures().join("customer-triage.loom"))
        .unwrap()
        .replacen("from: input.value", "from: input", 1);
    let err = from_str(&text).unwrap_err().to_string();
    assert!(err.contains("wires[0].from"), "unhelpful error: {err}");
    assert!(err.contains("node.port"), "unhelpful error: {err}");
}
