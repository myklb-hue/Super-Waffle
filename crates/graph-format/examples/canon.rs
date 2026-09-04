//! Prints the canonical form of a `.loom` file, so a fixture that is not yet
//! canonical can be diffed against what the writer would produce.
//!
//!     cargo run -p graph-format --example canon -- fixtures/graphs/x.loom
fn main() {
    let path = std::env::args().nth(1).expect("usage: canon <file.loom>");
    let graph = graph_format::load(&path).unwrap_or_else(|e| panic!("{e}"));
    print!("{}", graph_format::to_string(&graph));
}
