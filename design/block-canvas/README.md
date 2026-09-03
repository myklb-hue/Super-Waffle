# Block Canvas — UI mockups

Design working files for a node-based visual programming shell: drag blocks
onto a canvas, wire their typed ports together, run the graph.

Not part of the `tandem` crate. Nothing here is compiled or shipped.

## Layout

- `build.mjs` — generates every artboard from one set of shared tokens and
  primitives, so the shell chrome stays identical across screens.
- `*.dc.html` — one artboard each (Design Component format).
- `canvas.json` — artboard positions, pages, sticky notes, launch view.

## Regenerating

```
node build.mjs
```

Then re-seed and republish the canvas with the `design` skill's helper. The
seeded output (`block-canvas.html`, ~2.8 MB) is generated and gitignored.

## Design system

Dark node studio. Space Grotesk for UI, JetBrains Mono for anything
technical (port names, commands, log lines, values).

Ports are typed and the type is the colour — that is the rule the whole
design rests on:

| type     | colour    | carries                          |
| -------- | --------- | -------------------------------- |
| `text`   | `#56c7d6` | prompts, stdout, any string      |
| `tools`  | `#e0a458` | a bundle of callable functions   |
| `data`   | `#a78bd0` | structured json or a record      |
| `stream` | `#6fc98a` | output arriving incrementally    |
| `file`   | `#7f93c9` | a path or blob on disk           |
| `exec`   | `#e8ebf0` | control flow, never a value      |
| `any`    | `#8a93a3` | accepts every type               |

Block categories carry their own colour, shared between the library shelf
and the block header: models cyan, capabilities amber, runtimes green,
data violet, control slate, human rose.

## Artboards

**Screens**

- `EmptyShell` — nothing built; the inspector falls back to graph settings.
- `Main` — mid-drag, wiring `toolbox.tools` into `llm.tools`. Incompatible
  ports dim, the valid target glows.
- `Running` — the graph in flight, console drawer open, run panel on the right.
- `Inspector` — the same 328px column under five different selections.
- `Library` — the six categories, all 27 blocks, and the port-type legend.
- `BlockAnatomy` — one labelled block, every run state, the wiring rules.

**Clickable**

- `Interactive` — click a block to swap the inspector; Run animates the graph.
