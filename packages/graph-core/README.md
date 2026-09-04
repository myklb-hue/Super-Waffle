# @cyberloom/graph-core

The pure part of the graph: the types, the geometry the wire router depends on,
and the type grammar that decides whether a wire is legal. No React, no DOM, no
engine — everything here is testable on its own.

```
npm test -w @cyberloom/graph-core
npm run gen        # from the root, after changing anything in Rust
```

## Rust owns the schema

`src/generated/` is written by `cargo run -p block-kinds --bin export-types`
and committed rather than gitignored, so the frontend builds without a Rust
toolchain and a schema change shows up in the diff.

- `schema.ts` — the `.loom` shape and the catalogue types.
- `catalogue.json` — every built-in kind with its ports and settings.
- `compatibility.json` — all one hundred ordered pairs of port types and
  whether a wire between them is legal.

## Two copies of one rule

`compat.ts` is a second implementation of `PortType::accepted_by` in
`crates/graph-format/src/types.rs`. It exists because the drag needs an answer
per frame in the browser, with no round trip to the engine.

Two implementations of one rule is a liability, so `compat.test.ts` checks this
one against every pair Rust exported. Change the rule in Rust, run `npm run
gen`, and the test tells you to change it here too. That check has already
caught one real divergence.

`geometry.test.ts` does the same job for numbers: it reads
`packages/ui/src/styles/tokens.css` and asserts the constants here match the
tokens the artboards are drawn from, so a header height cannot change in one
place and not the other.
