# Cyberloom

A desktop application for building programs by dragging typed blocks onto a
canvas and wiring them together. A block is a model, a runtime, a sense, a
memory store, an actuator, a control structure, or a piece of your own code. A
wire is a typed connection. A graph runs once, runs live against a stream of
events, or runs on a schedule.

Single-user, local-first, for Linux. You own the machine, the models, the tools
and the data.

## What it does

A **camera** takes frames. A **detector** reads them and reports what it saw. An
**orchestrator** — a language model — reads that report, remembers it, and
decides. A **toolbox** offers it a shell, a Python interpreter and a servo
controller; a **memory hub** offers it what it knew yesterday; an **avatar**
gives it a face. None of that leaves your machine unless you turn a switch off.

Everything is a block, and the blocks are the same shape whether they hold a
model or a for-loop:

```
Webcam ──frames──▶ Object detection ──objects──▶ Orchestrator ──text──▶ Display
                                                      ▲   │
                          Memory hub ──memory─────────┘   └──▶ Text to speech ──▶ Avatar
```

A **custom block is a Python function**, and its signature is its interface: a
parameter without a default becomes an input port typed by its annotation, one
with a default becomes a setting with the right control, and the return
annotation becomes the output. Edit the code, and the block's shape changes
under your hands.

```python
def door_check(frame: Image, threshold: float = 0.6) -> Data:
    """Is the front door open?"""
    score = detect(frame)
    return {"open": score > threshold}
```

## Ten things worth knowing

1. **A wire carries a type.** Ten of them. The grammar refuses a bad connection
   before the wire exists rather than reporting it afterwards, and the ports
   that cannot accept what you are dragging dim while you drag it.
2. **`tools` and `memory` are handles, not flows.** The holder calls; the reply
   comes back on the same call. Everything else is one-way.
3. **A device that can be commanded can also report** — three ways at once: the
   tool's reply, telemetry on a `stream`, and a `fault` that interrupts. A fault
   wired into a Toolbox's `pause` stops tool calls before the model has finished
   its next thought.
4. **Warn, never block.** The application may warn before a dangerous action; it
   may not prevent one. Every prompt has a Continue.
5. **Frames and audio never leave the machine** and are not recorded unless you
   turn recording on. Faces are stored as embeddings, never images. Delete a
   person and every sighting goes with them.
6. **A capture nobody asked to keep goes away with the run**, because it was
   written into a folder that is deleted when the run ends. Privacy as a
   property of the program rather than a promise about it.
7. **The file is one readable YAML document** — `.loom` — with a canonical
   emitter, so a graph round-trips byte for byte and a diff is a diff.
8. **A workspace is a folder.** No project file, no import step. Its graphs are
   the tabs; its library, rigs and settings are its own.
9. **A rig is a folder too.** Four ship — Line, Robot, Orb, Pixel — and the
   avatar's vocabulary is generated from whichever one is worn, so a model
   driving an 8 × 8 matrix is never offered an expression a matrix cannot make.
10. **Secrets live in the OS keyring.** Only the name is written to the file.

## Running it

On CachyOS, or anything else with `pacman`, one script installs the
dependencies, builds, and puts Cyberloom in your application menu:

```sh
scripts/install.sh            # into ~/.local, no root needed beyond pacman
scripts/install.sh --system   # into /usr/local instead
```

Run it again after pulling to upgrade; `--uninstall` takes it out again. See
[`packaging/`](packaging/) for what it installs where.

To work on it rather than just run it:

```sh
npm ci
npm run gen          # Rust types → TypeScript, and the catalogue
cargo build -p loomd # the engine the dev server starts as a child process
npm run dev          # the shell, with the engine as a child process
```

`npm run dev` serves at `http://127.0.0.1:5173`; `?graph=door-watch` opens a
fixture, `?pick` opens the workspace picker. For the real window:

```sh
npx @tauri-apps/cli@2 dev
```

### Building a package

```sh
npx @tauri-apps/cli@2 build     # an AppImage, with the rigs inside it
```

See [`packaging/`](packaging/) for the AppImage launcher, the AUR `PKGBUILD`,
and what the first run does when nothing is installed yet.

### Tests

```sh
cargo test            # the engine, the format, the catalogue, the parsers
npm test              # the shell's stores and the type grammar
cargo clippy --all-targets --all-features
```

## How it is put together

Two processes. The **engine** owns everything that is true about a graph; the
**shell** owns everything about looking at one. They speak one JSON object per
line, so the same engine serves a window, a socket and — later — a headless
deployment, without any of them knowing about the others.

| Where | What |
| --- | --- |
| `crates/graph-format` | reads and writes `.loom`, with a hand-written canonical emitter |
| `crates/block-kinds` | the fifty built-in blocks: ports, settings, defaults, validation |
| `crates/block-source` | a custom block's signature, parsed into an interface |
| `crates/loomd` | the engine: plan, run, sources, senses, perception, memory, actuators, rigs |
| `apps/desktop` | the shell: canvas, inspector, library, transport, console |
| `apps/desktop/src-tauri` | the host: a window and a child process, and nothing else |
| `packages/ui` | the primitives, with a Ladle story for every state |
| `packages/graph-core` | geometry and the type grammar, shared by shell and tests |
| `rigs/` | four avatar rigs: a manifest and one SVG per expression |
| `fixtures/graphs` | four worked examples, used by the tests and by the app |

Anything the engine cannot do on the machine it is on, it does through a trait
with two implementations — a real one and a scripted one. That is how models,
perception, capture and serial devices all work, and it is why a graph with a
camera and a servo in it can be run, and tested, on a laptop with neither.

## Where things are written down

- **[`design/cyberloom/SPEC.md`](design/cyberloom/SPEC.md)** — the
  specification. Every design decision, the block catalogue, the type system,
  the run modes, custom blocks, the privacy rules, worked examples.
- **[`docs/PLAN.md`](docs/PLAN.md)** — the build plan, and after each slice a
  list of what building it turned up: the bugs, the decisions worth recording,
  and what could not be proven on this machine and why.
- `design/cyberloom/` — the mockups as working files, and the script that
  generates them from shared tokens.

## Status

All twelve slices of the build order are done: the format, the canvas, editing,
the engine, custom blocks, live graphs, senses, memory, actuators, the avatar,
the workspace, and packaging.

What is genuinely proven, and what is not, is written down slice by slice at the
end of `docs/PLAN.md`. The short version: everything above the hardware is
tested — 290 Rust tests and 71 TypeScript ones — and three things need a machine
this was not built on. There is no camera or servo here, so `lavfi:` and a
scripted controller stand in through the same code paths that open `/dev/video0`
and a serial port. The network policy denies the model hosts, so perception has
its interface and its wiring proven and its weights unfetched. And the AppImage
was built and its contents checked, but a fresh CachyOS install running the
assistant fixture from it is the one acceptance that needs CachyOS.

## Licence

MIT. See [`LICENSE`](LICENSE).
