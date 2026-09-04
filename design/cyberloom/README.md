# Cyberloom — UI mockups

Design working files for Cyberloom: drag blocks onto a canvas, wire
their typed ports together, run the graph.

## Master spec

`SPEC.md` is the master specification (draft 0.1, for approval). It
consolidates every decision from the design sessions and wins over any
mockup where the two disagree. `node build-spec.mjs` renders it to
`spec.html` with the figures under `fig/` embedded; that output is
generated and gitignored.

## Layout

- `SPEC.md` — the specification. `build-spec.mjs` renders it; `fig/` holds
  the rendered artboards it embeds.
- `build.mjs` — generates every artboard from one set of shared tokens and
  primitives, so the shell chrome stays identical across screens.
- `*.dc.html` — one artboard each (Design Component format).
- `canvas.json` — artboard positions, pages, sticky notes, launch view.

## Regenerating

```
node build.mjs
```

Then re-seed and republish the canvas with the `design` skill's helper. The
seeded output (`cyberloom.html`, ~2.8 MB) is generated and gitignored.

## Design system

Dark node studio. Space Grotesk for UI, JetBrains Mono for anything
technical (port names, commands, log lines, values).

Ports are typed and the type is the colour — that is the rule the whole
design rests on:

| type     | colour    | carries                                   |
| -------- | --------- | ----------------------------------------- |
| `text`   | `#56c7d6` | prompts, stdout, any string               |
| `tools`  | `#e0a458` | one callable, or a Toolbox bundle of them |
| `memory` | `#7e9ff0` | a store the model reads and writes        |
| `data`   | `#a78bd0` | structured json or a record               |
| `stream` | `#6fc98a` | output arriving incrementally             |
| `image`  | `#d77bd0` | frames from a camera or a file            |
| `audio`  | `#dcc65b` | samples from a microphone                 |
| `file`   | `#7f93c9` | a path or blob on disk                    |
| `exec`   | `#e8ebf0` | a trigger or control flow, never a value  |
| `any`    | `#8a93a3` | accepts every type                        |

Two bundling blocks follow the same shape: a **Toolbox** takes any number
of `tools` inputs and exposes one `tools` output; a **Memory hub** does the
same for `memory`. A runtime or actuator can also wire straight into
`llm.tools` for a simple run.

`tools` and `memory` wires are *handles*, not flows: the holder (the LLM,
or a Toolbox) calls, the device replies on the same call. They carry a
two-way mark at the holder's end. Anything a device reports on its own
initiative leaves on a separate port: `stream` or `data` for telemetry,
`exec` for a fault or interrupt. The Motors block on the assistant screen
shows all three.

**Warn, never block.** The user owns their tools. A truly dangerous action
(a shell command, a motor move) gets a warning prompt, and the prompt always
has a Continue. There is no approval gate, no admin lock, nothing the graph
cannot do. The "Warn before" toggles in the inspectors are the user's own
preference, not a permission.

**Custom blocks** are code. A parameter without a default is an input port
typed by its annotation; a parameter with a default is a setting in the
inspector; the return annotation is the output port; the docstring is the
description. The source is inline or a watched file, and a reload keeps
every wire whose port still exists.

The code is one *view* of a custom block, not the block. Each block shows
as Compact (name and ports), Summary (settings and description) or Code
(inline editor), switched from a toggle in its header and remembered per
block per graph. A big program lives in the code drawer under the canvas,
beside Console and Trace, or in the user's own editor via File mode, while
the block on the canvas stays small.

Ports are rows: inputs down the left edge, outputs down the right, one row
per index, above the block's body. Row *i* is centred 51 + 24·*i* px from
the block's top, which is what `build.mjs` uses to route wires.

Block categories carry their own colour, shared between the library shelf
and the block header: models cyan, capabilities amber, runtimes green,
senses yellow, memory blue, actuators orange, data violet, control slate,
human rose.

## Artboards

**Screens**

- `EmptyShell` — nothing built; the inspector falls back to graph settings.
- `Main` — mid-drag, wiring `toolbox.tools` into `llm.tools`. Incompatible
  ports dim, the valid target glows.
- `Running` — the graph in flight, console drawer open, run panel on the right.
- `Inspector` — the same 328px column under five different selections.
- `Library` — the six categories, all 27 blocks, and the port-type legend.
- `BlockAnatomy` — one labelled block, every run state, the wiring rules.

**Live and embodied**

- `Continuous` — a graph that never finishes: three source blocks keep it
  armed, a Loop frame repeats a region per item, the transport reads *live*,
  and the inspector with nothing selected becomes the run-mode panel.
- `RunModes` — the four transport states, plus panels for a source block, a
  Schedule, and a Loop frame.
- `Assistant` — a home assistant as one graph, library collapsed to a rail:
  webcam and microphone feed specialist models, which feed an orchestrator;
  memory stores bundle through a hub; text goes to a display and a speaker,
  thoughts to a terminal, actions to motors via an approval-gated tool call.
- `SensePanels` — inspector panels for a Webcam, Face recognition, the
  Memory hub, and Motors. Each leads with the boundary that matters
  (privacy, enrolment, what is stored, physical limits).
- `Avatar` — the Avatar panel, four rigs (Line, Robot, Orb, Pixel) running
  the same six expressions, and the command vocabulary.
- `AssistantStage` — the assistant graph with the Avatar in Stage view,
  resized from the corner, wires unmoved.

**Custom blocks**

- `CustomBlock` — a custom block open for editing on the canvas: inline
  code with the derived interface underneath, and an inspector whose
  Settings section was generated from the code's default arguments.
- `CustomRules` — the same code annotated line by line against the block it
  produces, the reload and error rules, and the Python, TypeScript and
  shell spellings.
- `CustomDrawer` — a 184-line block shown in Summary view on the canvas
  while its code is edited in the full-width drawer below.
- `BlockViews` — a custom block in Compact, Summary and Code beside a
  visual block in Compact, Summary and Stage; switching, resizing, and the
  three answers for big programs.

**Clickable**

- `Interactive` — click a block to swap the inspector; Run animates the graph.
