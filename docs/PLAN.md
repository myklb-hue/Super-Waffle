# Cyberloom — Build Plan

**Draft for review · 4 September 2026 · derived from SPEC v1.0**

This is the plan to build what `design/cyberloom/SPEC.md` describes.
It stops short of code: nothing here is scaffolded. Where the spec is
the *what*, this is the *how* and the *in what order*. §7 lists every
assumption I had to make; argue with those first.

## Stack and constraints

You left the stack and constraints for me to propose. Proposal, with the
reasoning, then everything below assumes it.

**Stack**

| Layer | Choice | Why |
| --- | --- | --- |
| Shell (desktop app) | **Tauri 2** on Linux (WebKitGTK), Rust host | Native window, small footprint, Rust host process for device and file access, and your other project is Rust so the toolchain is already on the machine. Electron would carry Chromium for no gain here. |
| Frontend | **React 19 + TypeScript + Vite** | The mockups are component-shaped; React's ecosystem has the one library that matters (below). |
| Canvas engine | **@xyflow/react** (React Flow 12) with custom nodes and edges | Pan/zoom, drag, selection, minimap, handles and edge routing are solved problems. Every visual rule in the spec (port geometry, bezier formula, handle marks, stage view) is achievable in custom node and edge components. Writing this from scratch is the single biggest schedule risk on the project; the risk with xyflow is only that its defaults have to be overridden precisely. |
| Styling | **CSS custom properties for tokens + CSS Modules**, no Tailwind | The spec's tokens are a small closed set (§16.1). Tailwind would mean expressing every 9.5 px mono label as a utility; modules keep component CSS next to the component and the tokens in one file. This is the one place I departed from the example stack in your brief; see §7. |
| Client state | **Zustand** with `immer` and `zundo` (undo) | One document store per open graph, one UI store, one server-cache store. Small, no boilerplate, undo for free. |
| Server cache | **TanStack Query** over Tauri commands and events | Runs, events, library, devices, models are engine-owned; Query gives caching, invalidation and subscriptions without a hand-written layer. |
| Engine | **Rust crate `engine`**, run as a separate process `loomd` | The graph runtime, block runtimes, device access, model clients. Separate from the shell from day one so Deploy-as-service (§15.1) is the engine without the shell. |
| Shell ↔ engine | **Local Unix socket, JSON-RPC 2.0 with a subscription channel** | Tauri talks to it through a thin Rust client in the host process; the headless service is the same binary with no client attached. |
| Types | **Rust is the source of truth**, TypeScript generated with `specta` / `tauri-specta` | The `.loom` schema, block manifests and RPC shapes are `serde` structs; the frontend never hand-writes a type the engine also owns. |
| Block runtimes | Python via a `uv`-managed venv per workspace, subprocess with a JSON line protocol; TypeScript/JavaScript via Bun (type-stripping, fast start); shell via `/bin/sh` | §10.5 and §15.8. |
| Models | **Ollama** HTTP for LLMs and embeddings; **whisper.cpp** for speech to text; **piper** for text to speech; **ONNX Runtime** for object detection (YOLOv8n) and affect; **insightface** via the Python runtime for faces | All local, all Linux-native, all with GPU paths on CachyOS. |
| Devices | V4L2 via `nokhwa`; PipeWire via `cpal`; serial via `serialport`; GPIO via `gpio-cdev` | §15.11. |
| Storage | `.loom` YAML files in the workspace folder; SQLite (`rusqlite`) + `sqlite-vec` for long-term memory; a per-workspace `.cyberloom/` folder for run logs and caches | §15.3, §9.2. |
| Packaging | AppImage first; AUR `PKGBUILD` second (natural for CachyOS); Flatpak later if wanted | |
| Tests | Vitest + Testing Library for components; Playwright for the shell; `cargo test` with fixture graphs for the engine; a golden-file test that a `.loom` round-trips byte-identically | |

**Constraints (proposed, non-negotiable unless you say otherwise)**

- **Single user, no auth.** One person, their machine. No accounts, no tokens in the shell.
- **Local-first, no telemetry.** Nothing leaves the machine unless a graph's *Local only* switch is off and the user has continued through the warning (§15.4).
- **Warn, never block** (§12.1) is enforced at the engine boundary: the engine never refuses a call; it emits a `warning` event that the shell renders with *Continue*.
- **Deploy target:** Linux desktop, Wayland and X11, CachyOS reference. No macOS or Windows work in v1, but nothing chosen above precludes them.
- **Browser support:** the Tauri webview only (WebKitGTK ≥ 2.44). No cross-browser CSS.
- **Typed ports are the grammar** (§4). Type compatibility is one pure function shared by shell and engine via the generated types; the shell never invents a rule the engine does not check.
- **Minimum window:** 1280 × 800; reference 1560 × 900; the library rail collapses below 1400 px wide.

---

## 1. Design tokens

Extracted from `design/cyberloom/build.mjs` (the `C`, `T`, `CAT`
tables and the dimensions the artboards use). Values are what the
mockups actually render. Where the mockups are inconsistent, the column
on the right is what to standardise on; the token file is written to the
standard, and the mockups are regenerated to match as part of slice 0.

### 1.1 Colour

Surfaces and text (single dark theme; §16.1):

| Token | Value | Use |
| --- | --- | --- |
| `--ground` | `#08090b` | window background |
| `--canvas` | `#0d0f13` | canvas; dot grid `rgba(255,255,255,.055)` at 22 px |
| `--panel` | `#111419` | library, inspector, drawers |
| `--bar` | `#0f1217` | top bar |
| `--block` | `#191d24` | block body |
| `--field` | `#0b0d11` | inputs, code, rows, stage background |
| `--line` | `#242932` | borders |
| `--line-soft` | `#1a1e25` | hairlines inside panels, block header rule |
| `--text-hi` / `-mid` / `-low` / `-faint` | `#e8ebf0` / `#98a2ae` / `#5f6875` / `#39414c` | four-step text ramp |
| `--accent` | `#56c7d6` | selection ring, primary button, active tab, segmented control |
| `--ok` / `--warn` / `--err` | `#6fc98a` / `#e0a458` / `#e0685f` | status; `--err` also tints safety and privacy sections |

Port types (§4.1) and categories (§6). Note that five of the category
colours are the same value as a type colour by design (models = text,
capabilities = tools, runtimes = stream, memory = memory, data = data):

| Type | Value | | Category | Value |
| --- | --- | --- | --- | --- |
| `text` | `#56c7d6` | | models | `#56c7d6` |
| `tools` | `#e0a458` | | capabilities | `#e0a458` |
| `memory` | `#7e9ff0` | | runtimes | `#6fc98a` |
| `data` | `#a78bd0` | | senses | `#dcc65b` |
| `stream` | `#6fc98a` | | memory | `#7e9ff0` |
| `image` | `#d77bd0` | | actuators | `#e8865a` |
| `audio` | `#dcc65b` | | data | `#a78bd0` |
| `file` | `#7f93c9` | | control | `#8a93a3` |
| `exec` | `#e8ebf0` | | human | `#d97f8f` |
| `any` | `#8a93a3` | | custom | `#c3ccd8` |

Alpha tints are derived, never separate tokens: `rgba(colour, .12)` for
chip fills, `.14` for icon wells, `.13` → `.02` gradient for block
headers, `.09` for wire halos, `.16` for live halos, `.3` for dimmed
ports.

**Inconsistencies found**

| Where | Mockup | Standardise on |
| --- | --- | --- |
| `file` type `#7f93c9` vs `memory` type `#7e9ff0` | Two blues 2° apart in hue; distinguishable side by side, never on a wire | Keep `memory` `#7e9ff0`; move `file` to `#93c76b`, the one colour in the yellow-green gap between `audio` and `stream` and 44° from each. A first attempt at slate-teal `#6fa3a8` was rejected in slice 0: it landed 2° from `text`, trading one same-hue collision for another. |
| Console log text `#b3bcc7`, code text `#c3cad4`, panel prose `#c3cad4` | Three near-identical off-whites | One `--text-body` `#c3cad4`. |
| Terminal-thoughts background `#07080a` vs `--field` `#0b0d11` | Ad hoc darker field | Use `--field`; the terminal is not darker than the code editor. |
| Zoom pill / drag tooltip backgrounds `rgba(#12161c,.92)` and `.96` | Ad hoc floating surface | One `--float` `#12161c` at `.94`. |

### 1.2 Type

Two families (§16.2): **Space Grotesk** 400–700 for UI, **JetBrains
Mono** 400–700 for anything technical. Both from Google Fonts in the
mockups; **bundle them** in the app (WOFF2, OFL licence) so the shell
works offline.

Sizes the mockups use, in px: 7, 9, 9.5, 10, 10.5, 11, 11.5, 12, 12.5,
13, 13.5, 16.5, 19, 20, 24, 40. That is too many. Proposed scale:

| Token | px | Role | Replaces |
| --- | --- | --- | --- |
| `--fs-xs` | 10 | mono port labels, chips, section labels, log timestamps, minimap | 9, 9.5, 10 |
| `--fs-sm` | 11 | mono values, console, code, panel hints, list meta | 10.5, 11 |
| `--fs-md` | 12 | block titles, panel body, fields, library rows, tabs | 11.5, 12 |
| `--fs-lg` | 13 | panel titles, sheet captions | 12.5, 13, 13.5 |
| `--fs-xl` | 20 | sheet titles (design docs only, not in the app) | 19, 20 |

The 7 px labels inside the camera preview are illustration, not UI.
Line heights: 1.45 for prose, 1.6 for mono blocks, 19 px fixed for code
lines. Tracking: `.12em` on uppercase mono section labels, `.03em` on
port labels, `-.01em` on titles.

**Inconsistency:** the port label is 9.5 px in every block and 10 px in
the library rows; the chip is 9.5 px everywhere. Standardise both to 10.
Every block gets 0.5 px taller in label rows; the port row stays 24 px.

### 1.3 Spacing

Values used: 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 18, 22, 24,
26, 28, 32, 40, 48. Proposed scale, 4-based with two tight steps:

`--sp-1` 2 · `--sp-2` 4 · `--sp-3` 6 · `--sp-4` 8 · `--sp-5` 12 ·
`--sp-6` 16 · `--sp-7` 24 · `--sp-8` 32 · `--sp-9` 48

Mapping the odd values: 5 → 4 or 6 by context (gap between chips 4, gap
between rows 6); 7 → 8; 9 → 8; 10 → 8 (block body horizontal padding
becomes 8 + 4 for the port gutter, still reads as 12); 11 → 12; 13 →
12; 14 → 16 (section padding); 18 → 16; 22 stays as the grid unit only;
26/28 → 24 (control heights, below).

### 1.4 Radii

Used: 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 50%.

| Token | px | Use |
| --- | --- | --- |
| `--r-xs` | 3 | keycaps, small squares |
| `--r-sm` | 4 | chips, view toggle, icon buttons |
| `--r-md` | 6 | fields, buttons, connection rows, list rows |
| `--r-lg` | 9 | blocks (8 inside the border for the header) |
| `--r-xl` | 12 | loop frames, sheet cards, empty-state drop zone |
| `--r-full` | 999 | pills (start-from chips, hint pill), dots |

**Inconsistencies:** 5 (tool rows in the Toolbox body) → 4; 7 (align
buttons, rig thumbnails, stage-block corner) → 6 or 9; 10 (sheet cards)
→ 12; 13/14/15 (pills) → full.

### 1.5 Elevation

| Token | Value | Use |
| --- | --- | --- |
| `--shadow-block` | `0 10px 26px rgba(0,0,0,.45)` | every block at rest |
| `--shadow-float` | `0 8px 24px rgba(0,0,0,.5)` | zoom pill, minimap, drag tooltip |
| `--shadow-selected` | `0 0 0 1px var(--accent), 0 0 0 5px rgba(86,199,214,.15), 0 16px 38px rgba(0,0,0,.6)` | selected block |
| `--shadow-running` | `0 0 0 4px rgba(111,201,138,.09), 0 12px 30px rgba(0,0,0,.5)` | running block |
| `--ring-glow(colour)` | `0 0 0 4px rgba(c,.3), 0 0 16px rgba(c,.6)` | a port that accepts the current drag |

**Inconsistency:** the drag tooltip uses `0 8px 20px .6`; fold into
`--shadow-float`.

### 1.6 Dimensions and geometry

These are load-bearing: the wire router depends on them (§3.1).

| Token | Value |
| --- | --- |
| Top bar / status bar | 46 / 28 |
| Library / rail / inspector | 264 / 48 / 328 |
| Console drawer / code drawer | 176 / 300 |
| Block header (summary) / (stage) | 31 / 24 |
| Port row height / first port centre | 24 / 51 (`51 + 24·i`) |
| Port dot / halo / stage dot | 11 / 3 / 11 |
| Wire core / handle / halo | 1.9 / 2.2 / 5 |
| Bezier control offset | `max(48, .55·|Δx|, .22·|Δy|)` |
| Handle mark | two chevrons at `x2 − 22` and `x2 − 13`, 3.5 px tall |
| Block min width / max in mockups | 168 / 480 |
| Grid | 22 |
| Resize grip | 12, bottom-right, 3 px inset |
| Field height / small control / button | 30 / 26 / 28 |
| Chip height | 20 |
| Library row / category header | 28 / 24 |
| Avatar stage default | 240 × 240, aspect locked |

**Inconsistencies:**

| Where | Mockup | Standardise on |
| --- | --- | --- |
| Block header 31 px | Odd number; comes from 30 + 1 border | **32**, and the first port centre becomes **52**. Every wire endpoint moves 1 px; regenerate the artboards. |
| Control heights 26 (icon button), 28 (transport, buttons), 30 (fields) | Three heights | **28** for buttons and icon buttons, **32** for fields and segmented controls, so fields align with the 32 px header rhythm. |
| Tabs strip in panels 33 px (drawer) vs implicit ~34 (inspector) | | **32**. |
| Chip height 20 in blocks, 22 in the custom block's interface strip, 24 for the drag tooltip | | 20 for chips; the interface strip and tooltip are rows, 24. |

### 1.7 Breakpoints

The shell is not responsive in the web sense; it has three layouts by
window width:

| Width | Layout |
| --- | --- |
| ≥ 1560 | Reference: library 264, inspector 328 |
| 1400–1559 | Library collapses to the 48 px rail by default (user can pin it open) |
| 1280–1399 | Rail, and the inspector narrows to 300 with its section padding at `--sp-5` |
| < 1280 | Not supported in v1; the window's minimum size |

---

## 2. Component inventory

Props are given as TypeScript-ish signatures. "States" lists which of
loading / empty / error / disabled each needs; a component not listed
under a state has nothing to show for it.

### 2.1 Shared primitives (`packages/ui`)

Used by two or more screens or panels. No component here knows about
graphs, blocks or the engine.

| Component | Props | States |
| --- | --- | --- |
| `Icon` | `name: IconName; size?: 10\|11\|12\|13\|14\|18; color?: token; strokeWidth?` | — |
| `StatusDot` | `state: 'idle'\|'queued'\|'running'\|'ok'\|'error'\|'off'` | — (the state *is* the prop) |
| `Chip` | `label; color: token; dot?; solid?; size?: 'sm'\|'md'` | — |
| `TypeDot` / `TypeDots` | `kind: PortType` / `kinds: PortType[]` | — |
| `Field` | `value; icon?; mono?; muted?; select?; suffix?; onChange?; onOpen?` | disabled, error (red border + message), loading (skeleton) |
| `Slider` | `label; value; min; max; step; unit?; color?; onChange` | disabled |
| `Toggle` | `on; color?; onChange` | disabled |
| `SwitchRow` | `label; hint?; on; color?; onChange` | disabled |
| `Segmented` | `options: string[]; value; color?; onChange` | disabled |
| `Button` | `label; icon?; variant: 'primary'\|'default'\|'danger'; onClick` | disabled, loading (spinner replaces icon) |
| `Section` | `title; tint?: token; right?: ReactNode; children` | — |
| `Label` | `children` | — |
| `TextBox` | `value; minHeight?; mono?; onChange?` | disabled, empty (placeholder) |
| `ConnectionRow` | `icon; name; meta; kind: PortType; state: StatusDot state \| 'pending'` | — |
| `Tabs` | `tabs: string[]; active; onChange` | — |
| `PanelHeader` | `icon; color; title; sub; tabs?; active?; onTab?; menu?` | — |
| `KeyHint` | `keys: string` | — |
| `Callout` / `DashedHint` | `title; body; color?` | — |
| `EmptyState` | `icon; title; hint; actions?` | — (it *is* the empty state) |
| `Grip` | `onResize(dx, dy)` | disabled |
| `ViewToggle` | `active: View; third?: 'code'\|'stage'\|null; onChange` | — |
| `CodeView` | `lines: Token[][]; height?; marks?; fontSize?` | loading, empty ("no source"), error (a red line marker) |
| `Meter` | `bars: number[]; color` | — |
| `Tooltip` | `content; children` | — |
| `Kbd`-aware `Menu` | `items; onSelect` | disabled items |

### 2.2 Canvas components (`apps/desktop/src/canvas`)

Built on xyflow's custom node and edge APIs.

| Component | Props | States |
| --- | --- | --- |
| `BlockNode` (xyflow node) | `data: BlockView` (block, view, size, ports, selected, state, badges) | idle / queued / running / done / error / disabled / breakpoint (the block state); selected; hovered (shows toggle + grip); dimmed (during an incompatible drag) |
| `StageNode` | as `BlockNode` with `content: ReactNode; aspectLocked` | as above |
| `PortHandle` (xyflow handle) | `port: Port; side; index; dim?; glow?; labelHidden?` | dim, glow, snap |
| `PortZone` | `ins: Port[]; outs: Port[]; highlightRow?` | — |
| `Wire` (xyflow edge) | `kind: PortType; live?; pending?; handle?; dragging?` | live (animated dash), pending (28 % + short dash), dragging (dashed + glow) |
| `HandleMark` | `x; y; color` | — |
| `SnapRing` | `x; y; color` | — |
| `DragTooltip` | `from: PortRef; to: PortRef` | compatible / incompatible copy |
| `LoopFrame` (xyflow group node) | `frame: Frame; counters: {i, n, queue, parallel}; ports` | idle / running / error |
| `Minimap` | xyflow's, restyled with category colours | — |
| `ZoomPill` | `zoom; onZoom; onFit` | — |
| `HintPill` | `text` | — |
| `EmptyCanvas` | `templates: Template[]; onPick` | — |
| `Legend` | `entries: {kind, label}[]` | — |

Block body previews are per block kind and live with the block kind's
definition (§2.4), not here.

### 2.3 Shell components (`apps/desktop/src/shell`)

| Component | Props | States |
| --- | --- | --- |
| `TopBar` | `graph: GraphMeta; run: RunState; runtime: RuntimeStatus; zoom` | saved / edited / saving; transport in each of Run, running, live, next-in, paused |
| `Transport` | `mode: RunMode; run?: RunState; onRun; onStop; onPause; onStep` | as above; disabled while the engine is unreachable |
| `RuntimeChip` | `runtime: RuntimeStatus; localOnly` | reachable / unreachable / starting |
| `GraphTabs` | `graphs: GraphMeta[]; active; onSelect; onClose` | dirty marker; a subgraph tab shows its breadcrumb |
| `LibraryPanel` | `categories; placed: Set<BlockKind>; query; onDrag; onSearch` | loading (skeleton rows), empty (no search hits), error (library failed to load — rare, shown inline) |
| `LibraryRail` | `categories; onExpand` | — |
| `CategoryHeader` | `category; open; count; onToggle` | — |
| `BlockRow` | `entry: LibraryEntry; placed?; dragging?` | placed |
| `Inspector` | `selection: Selection; graph; run?` — routes to a panel | loading (engine data pending), empty (never: nothing-selected is the graph panel) |
| `StatusBar` | `left: string; right: string; warnings: number` | — |
| `Drawer` | `tabs: DrawerTab[]; active; height; onResize; onClose` | collapsed / open |
| `ConsoleView` | `lines: LogLine[]; filter; warnings` | empty ("no output yet"), streaming |
| `TraceView` | `events: RunEvent[]` | empty |
| `VariablesView` | `vars: Record<string, unknown>` | empty |
| `CodeDrawer` | `file: SourceFile; interface: Interface; onOpenExternal` | loading, error (parse error with line), unsaved |
| `WarningPrompt` | `warning: Warning; onContinue; onSuppress` | — |
| `CommandPalette` (`⌘K`) | `entries; onPick` | empty |

### 2.4 Inspector panels (`apps/desktop/src/panels`)

One component per panel in §7.4, composed from the primitives. Each
takes its subject and an `onChange` that writes to the document store;
live sections take data from the run cache.

| Panel | Subject | Needs from engine | States |
| --- | --- | --- | --- |
| `GraphPanel` | `Graph` | armed sources, recent events, runtime status | run mode in each of Once/Live/Schedule |
| `RunPanel` | `Run` | progress per block, live output, usage | running / paused / finished / failed |
| `LlmPanel` | `Block<'llm'>` | model list from Ollama, connected tools and memory | models loading, provider unreachable (error), orchestrator role adds Specialists/Memory/Thoughts sections |
| `TerminalPanel` | `Block<'terminal'>` | last run | — |
| `ToolboxPanel` | `Block<'toolbox'>` | exposed functions from connected blocks | empty (no tools connected) |
| `InputPanel` | `Block<'input'>` | — | — |
| `WirePanel` | `Wire` | last payload (watch) | no value yet |
| `MultiPanel` | `Block[]` | — | — |
| `SourcePanel` (watch folder, webhook) | `Block<source>` | rate, totals | armed / listening / paused |
| `SchedulePanel` | `Block<'schedule'>` | next tick | armed / disabled |
| `LoopPanel` | `Frame` | iteration, queue | idle / running |
| `WebcamPanel` | `Block<'webcam'>` | device list, live preview frame | no camera (empty), device busy (error) |
| `FacePanel` | `Block<'facerec'>` | known people from memory | empty (no people enrolled) |
| `MemoryHubPanel` | `Block<'memhub'>` | store sizes | empty (no stores wired) |
| `MotorsPanel` | `Block<'motors'>` | position, pending | device disconnected (error) |
| `CustomBlockPanel` | `Block<'custom'>` | parsed interface, parse errors, file watch state | parse error (red, with line), file missing (error), reloading |
| `AvatarPanel` | `Block<'avatar'>` | current expression, rig list | rig failed to load (error) |

Every panel with a third view includes the shared `ViewSection`
(`view; size; aspectLocked?`).

### 2.5 Block kinds (`packages/blocks` — definitions, not UI)

Not components, but the inventory the components render from. Each
block kind is a definition object: `{ kind, category, icon, title,
ports: PortDef[], settings: SettingDef[], views: View[], body:
(block, live) => ReactNode, panel: Component }`. The library, the
canvas and the inspector all read from this one table; a new block kind
is one file. The 44 built-ins from §6 plus Convert (§15.5) and Subgraph
(backlog) live here. The `body` renderer is the only UI in the package.

### 2.6 Screen-specific components

| Component | Screen | Props | States |
| --- | --- | --- | --- |
| `RigPicker` | Avatar panel, rig sheet | `rigs: Rig[]; active; onPick; onAdd` | loading, add-rig error |
| `RigFace` | Avatar block body, stage, panel | `rig: RigId; expression; intensity?; gaze?; speaking?; size` | rig missing → placeholder face |
| `VocabularyChips` | Avatar panel | `commands: string[]` | — |
| `InterfaceRows` | Custom block panel and block strip | `iface: Interface; changes?: Diff` | parse error |
| `AffectChips` | Affect block body | `affect: {label, p}[]` | — |
| `CamPreview` | Webcam body and panel | `frame?: ImageBitmap; boxes?: Box[]` | no frame (dark placeholder) |
| `TokenMeter` | LLM body while running | `tokens; rate; progress` | — |
| `ProgressList` | Run panel | `steps: {block, state, ms}[]` | — |
| `RecentEvents` | Graph panel (live) | `events: SourceEvent[]` | empty |
| `KnownPeople` | Face panel | `people: Person[]; onName; onDelete` | empty |
| `TransportCard` | RunModes sheet only (design doc) | — | — |

---

## 3. Screens and routes

A desktop app, so "routes" are the shell's top-level states, held in
the URL of the webview for deep links and reload survival
(`canvas://workspace/<path>?graph=<id>&tab=<id>`), not a website.

| Route | Screen | Data it needs |
| --- | --- | --- |
| `/` | **Workspace picker**: recent workspaces, open folder, new | `RecentWorkspace[]` from app settings |
| `/w` | **Workspace**: graph tabs + canvas + panels (the shell of §2) | `Workspace`, `GraphMeta[]`, `Library`, `RuntimeStatus` |
| `/w?graph=<id>` | **Graph** (Figures 1, 3, 7, 8, 9, 16) | `Graph` (document), `Run?` (live), `Selection` (UI) |
| `/w?graph=<id>&sub=<id>` | **Subgraph tab** (backlog) | as Graph, scoped |
| `/w?graph=<id>&drawer=code:<block>` | **Code drawer open** (Figure 13) | `SourceFile`, `Interface` |
| `/w?graph=<id>&run=<id>` | **Run in flight** (Figure 7): same screen, Run panel and transport in run state | `Run`, `RunEvent` stream |
| `/w/library` | **Library management** (backlog): custom blocks, rigs, models | `LibraryEntry[]`, `Rig[]`, `ModelInfo[]` |
| `/w/settings` | **Workspace settings**: runtime, Python env, devices, local-only default | `WorkspaceSettings`, `DeviceInfo[]` |
| `/presence?graph=<id>` | **Presence view** (backlog, §15.9): the Avatar stage plus transport, for small screens and the deployed state | `Run`, `AvatarState` |

### 3.1 Data shapes

Rust structs, TypeScript generated. Shown as TypeScript here for
reading. `Id` is a short random string, stable across saves.

```ts
type PortType = 'text'|'tools'|'memory'|'data'|'stream'|'image'|'audio'|'file'|'exec'|'any';
type View = 'compact'|'summary'|'code'|'stage';
type BlockState = 'idle'|'queued'|'running'|'done'|'error'|'disabled'|'breakpoint';

interface Graph {                // one .loom file
  version: 1;
  id: Id; name: string; description?: string;
  runMode: 'once'|'live'|'schedule';
  localOnly: boolean;            // §15.4, default true
  execution: { runtime: 'local'; concurrency: number; timeoutSec: number };
  defaults: { provider: string; model: string };
  overlap: { policy: 'queue'|'dropNewest'|'dropOldest'|'coalesce'; maxQueue: number; coalesceMs: number; loopParallel: number };
  between: { keepState: boolean; restartOnCrash: boolean };
  env: Record<string, string>;   // secret *names*; values live in the OS keyring
  blocks: Block[];
  frames: Frame[];
  wires: Wire[];
  subgraphs?: Graph[];           // inline, §15.2
  ui: { viewport: { x: number; y: number; zoom: number } };
}

interface Block {
  id: Id; kind: string;          // 'llm' | 'terminal' | 'custom' | ...
  title?: string;                // override of the kind's title
  position: { x: number; y: number };  // grid-rounded on save
  size?: { w: number; h?: number };    // h only for stage/code views
  view: View;
  settings: Record<string, unknown>;   // validated against the kind's SettingDef[]
  ports?: PortOverride[];        // custom blocks: the parsed interface
  source?: { mode: 'inline'|'file'; language: 'python'|'typescript'|'shell'; code?: string; path?: string };
  disabled?: boolean; breakpoint?: boolean;
  frame?: Id;                    // which loop frame contains it
}

interface Port { name: string; type: PortType; side: 'in'|'out'; optional?: boolean; setting?: boolean }
interface Wire { id: Id; from: { block: Id; port: string }; to: { block: Id; port: string } }
interface Frame { id: Id; kind: 'loop'; position; size; over: { block: Id; port: string }; as: string; parallel: number; max: number; stopWhen?: { block: Id; port: string }; continueOnError: boolean }

interface Run {                  // engine-owned
  id: Id; graph: Id; mode: 'once'|'live'|'schedule';
  state: 'running'|'paused'|'finished'|'failed'|'stopped';
  startedAt: string; elapsedMs: number;
  blocks: Record<Id, { state: BlockState; ms?: number; live?: unknown }>;
  usage: { tokensIn: number; tokensOut: number; cost?: number };
  eventsTotal: number; queueDepth: number;
}
type RunEvent =
  | { t: 'log'; at: number; source: Id|'graph'; level: 'info'|'warn'|'error'; text: string }
  | { t: 'block'; at: number; block: Id; state: BlockState; ms?: number }
  | { t: 'wire'; at: number; wire: Id; payload?: unknown }      // for live wires and watch
  | { t: 'value'; at: number; block: Id; preview: unknown }     // inline previews, throttled
  | { t: 'warning'; at: number; block: Id; warning: Warning }   // §12.1, needs Continue
  | { t: 'source'; at: number; block: Id; kind: string; summary: string }
  | { t: 'avatar'; at: number; block: Id; expression: string; speaking: boolean; gaze?: [number, number] };

interface Warning { id: Id; block: Id; action: string; detail: string; suppressible: boolean }

interface LibraryEntry { kind: string; category: string; title: string; icon: string; ports: Port[]; custom?: { path: string } }
interface Interface { ins: Port[]; outs: Port[]; settings: SettingDef[]; description?: string; parsedAt: number; errors: { line: number; message: string }[] }
interface Rig { id: string; name: string; path: string; expressions: string[]; gestures: string[]; idle: { blinkSec: [number, number]; breathePerMin: number; settleSec: number; sleepMin: number } }
interface RuntimeStatus { engine: 'starting'|'ready'|'unreachable'; ollama?: { reachable: boolean; models: string[]; gpu?: string }; python?: { version: string; venv: string } }
```

The YAML on disk is the same shape with keys in the order above; the
engine writes it, the shell never serialises YAML itself.

---

## 4. State ownership

Four kinds of state, four owners. The rule: **the engine owns anything
that happens; the shell owns anything that is drawn; the file owns
anything that persists.**

| State | Owner | Lives in | Persisted | Fetched / cached how |
| --- | --- | --- | --- | --- |
| **Document** — blocks, wires, frames, settings, views, sizes, viewport, run mode, local-only | Shell (document store) | Zustand + immer, one store per open graph, `zundo` for undo | `.loom` via engine `graph.save`, autosaved 300 ms after the last edit | Loaded once via `graph.open`; the engine validates and returns the canonical form; the shell never mutates a graph the engine has not accepted |
| **UI** — selection, hover, drag-in-progress, open drawer and tab, panel scroll, palette open, library search, pinned library | Shell (UI store) | Zustand, not undoable | Not persisted (except open tabs and drawer height in workspace settings) | — |
| **Derived** — port positions, wire paths, type compatibility, dimmed/glow sets during a drag, block heights | Shell, computed | Pure functions in `packages/graph-core`, memoised per block | No | Recomputed from document + UI |
| **Engine** — runs, events, block live state, inline previews, source-block counters, warnings, parsed custom-block interfaces, file-watch state | Engine | Rust; the shell holds a cache | Run logs to `.canvas/runs/<id>/`; nothing else | TanStack Query: `run.get` on open, `run.subscribe` streams `RunEvent`s over the socket into the query cache; previews throttled to 10 Hz by the engine |
| **Library** — built-in kinds, custom blocks in the workspace, rigs | Engine (scans the workspace) | Cache | The files themselves | `library.list`, invalidated on the engine's `library.changed` event (file watcher) |
| **Devices and models** — cameras, mics, serial ports, Ollama models, GPU | Engine | Cache | No | `devices.list`, `models.list`, refetched on focus and on `devices.changed` |
| **Long-term memory** — people, places, episodes | Engine (SQLite) | — | `.canvas/memory.sqlite` | Read only through panels (`memory.people`, `memory.episodes`), paginated |
| **App settings** — recent workspaces, window size, theme | Shell host | Tauri store plugin | `~/.config/cyberloom/` | Read at start |
| **Secrets** | OS keyring via the host | — | keyring | Never sent to the webview; the engine reads them by name |

Two consequences worth stating:

- **Undo is document-only.** Undoing a canvas edit never touches a run.
  A running graph edited on the canvas is live-reloaded by the engine on
  save, per the reload rules (§10.3); the Run panel shows "reloaded" as
  an event.
- **The shell can be closed and reopened against a running engine.** On
  `graph.open` the engine reports `runs.active`, and the shell resumes
  the subscription. This is also the whole of Deploy-as-service later.

Fetching detail: the Tauri host process holds one socket to `loomd`
and multiplexes it; the webview calls `invoke('rpc', {method, params})`
and listens to `rpc:event`. If `loomd` is not running the host starts
it (`loomd --workspace <path>`); the shell shows `engine: starting` in
the runtime chip and disables the transport until `ready`.

---

## 5. Folder structure

A single repository, Cargo workspace + npm workspaces.

```
super-waffle/
  apps/
    desktop/                 Tauri app
      src/                   React frontend
        app/                 routes, providers, the shell layout
        shell/               TopBar, Transport, GraphTabs, LibraryPanel, Inspector, StatusBar, Drawer
        canvas/              xyflow integration: BlockNode, StageNode, Wire, LoopFrame, ports, drag logic
        panels/              one file per inspector panel
        stores/              document.ts, ui.ts, queries.ts (TanStack), rpc.ts
        styles/              tokens.css, fonts/, globals.css
        generated/           types from specta — never edited
      src-tauri/             Rust host: window, socket client to loomd, keyring, settings
  crates/
    engine/                  graph model, validation, scheduler, run loop, events
    loomd/                   the daemon binary: socket server, workspace watcher, wraps engine
    graph-format/            .loom YAML read/write, round-trip tests
    block-kinds/             built-in kind definitions (ports, settings) shared by engine and generated TS
    runtime-python/          subprocess protocol, signature parsing (via a bundled parser script)
    runtime-js/              Bun protocol
    runtime-shell/
    models-ollama/           chat, embeddings, streaming
    models-onnx/             object detection, affect
    speech/                  whisper.cpp and piper bindings
    devices/                 v4l2, pipewire, serial, gpio
    memory/                  sqlite + vectors, consolidation
    avatar/                  rig loading (SVG states + rig.yaml), expression state machine, lip sync from amplitude
  packages/
    ui/                      primitives (§2.1), Ladle stories
    graph-core/              pure TS: geometry, bezier, compatibility, layout helpers; tested against fixtures
    blocks/                  block kind UI: body renderers and panel registry (§2.5)
  rigs/
    line/  robot/  orb/  pixel/     rig.yaml + one SVG per state
  fixtures/
    graphs/                  customer-triage.loom, inbox-triage.loom, home-assistant.loom, door-watch.loom
  design/                    the mockups and SPEC.md (existing)
  docs/
    PLAN.md                  this document
    ADR/                     one file per decision that changes after v1.0
```

**Naming**

- React components: `PascalCase.tsx`, one exported component per file, with
  `PascalCase.test.tsx` beside it. Styles are colocated per *set* rather than
  per file: the primitives share `packages/ui/src/components/ui.module.css`
  because they are atoms over the same dozen tokens, while a component with
  real internal layout (a canvas node, an inspector panel) gets its own
  `PascalCase.module.css`. CSS Modules scopes the names either way.
- Hooks `useThing.ts`; stores `thing.ts` exporting `useThingStore`;
  pure modules `kebab-case.ts`.
- Block kinds: `packages/blocks/src/kinds/<kind>.tsx` exporting
  `const llm: BlockKind`; the registry is the folder index.
- Rust: crates `kebab-case`, modules `snake_case`, one `mod.rs` per
  feature area, no `lib.rs` longer than the exports.
- Generated types: `apps/desktop/src/generated/` is gitignored and
  regenerated by `cargo run -p loomd -- export-types` in the dev script.
- Fixture graphs are named for their example in the spec (§13).
- CSS tokens are `--kebab` with the prefixes in §1; no component defines
  a colour literal.

---

## 6. Build order

Vertical slices. Each ends with something runnable and reviewed. The
figure numbers are the spec's; "done" means the slice matches its
figure.

| # | Slice | Depends on | Done when |
| --- | --- | --- | --- |
| 0 | **Tokens and primitives.** `tokens.css`, fonts bundled, every §2.1 primitive in Ladle with its states. Regenerate the mockups from the standardised tokens (§1) so spec and app agree. | — | Ladle shows every primitive; the artboards re-render with the 32 px header. |
| 1 | **Graph format and types.** `graph-format` reads and writes `.loom`; `block-kinds` defines the 49 built-ins; `specta` exports TS; `graph-core` has geometry and compatibility with tests against the four fixture graphs. | 0 | `customer-triage.loom` round-trips byte-identical; `compat('data','text')` and friends match §4.1. |
| 2 | **Static canvas.** Tauri window, `loomd` starts and serves `graph.open`; xyflow renders blocks, ports and wires from a fixture with the spec's geometry; minimap, zoom pill, library panel (read-only), status bar. No editing. | 1 | Figure 1 and Figure 3's block layout (without the drag) pixel-match. |
| 3 | **Editing.** Drag from library, move, delete, wire drag with dim/glow/snap/tooltip, selection ring, graph and block inspectors for Input/LLM/Terminal/Toolbox, wire and multi panels, view toggle and grip, undo/redo, autosave. | 2 | Figure 3 including the drag; Figure 5 all five panels; a graph built by hand saves and reopens identically. |
| 4 | **Engine v0 and running.** Scheduler for Once mode; LLM via Ollama with streaming; Terminal and Python runtimes; Toolbox bundling and tool calls; warnings with Continue; console drawer; Run panel; live wires and inline previews. | 3 | Figure 7; the customer-triage example runs end to end against a local model. |
| 5 | **Custom blocks.** Python signature parsing, inline and file modes, reload rules, error rules, generated settings, Code view, code drawer, save to library; then TypeScript/JavaScript and shell. | 4 | Figures 10, 12, 13; `door_check` works. |
| 6 | **Live graphs.** Sources (schedule, watch folder, webhook), Live and Schedule modes, transport states, overlap policy, between-event state, loop frames, graph panel's run-mode sections. | 4 | Figures 8 and 14; inbox-triage runs live for an hour without intervention. |
| 7 | **Senses and perception.** Webcam (V4L2), microphone (PipeWire), keyboard; speech to text (whisper.cpp), text to speech (piper), speaker, display; object detection and affect (ONNX); face recognition (insightface via the Python runtime). Their panels. | 5, 6 | Figure 6's webcam and face panels; a webcam frame reaches an LLM as a detection. |
| 8 | **Memory.** Working memory, long-term memory (SQLite + vectors), Memory hub, `remember()`/`forget()` as tools, consolidation, the hub panel and its privacy section. | 4 | Figure 6's hub panel; the assistant remembers a name across runs. |
| 9 | **Actuators and feedback.** Serial and motors, `state` and `fault` ports, Toolbox `pause`, warn-before-move, motors panel. | 4 | Figure 9's motor chain; a fault pauses the Toolbox and one click resumes. |
| 10 | **Avatar.** Rig loader (SVG states + `rig.yaml`), the four rigs with seven expressions, expression state machine, idle behaviours, lip sync from the speech audio, gaze from `look`, Stage view and resize, Avatar panel and Rigs tab, Status light and Sound cue. | 7, 9 | Figures 9, 15, 16; the home-assistant fixture runs with a face. |
| 11 | **Workspace.** Workspace picker, graph tabs, per-workspace library, settings screen, Local-only switch and the remote-model warning, Convert-on-wire insert. | 3 | Two graphs open as tabs; a remote model warns once. |
| 12 | **Packaging.** AppImage, then AUR; Wayland and X11 checked on CachyOS; first-run experience (engine, Ollama, Python env detection with clear messages; model downloads explicit and resumable, offline a supported state). | 2 | A fresh CachyOS install runs the assistant fixture from the AppImage. |
| — | **Backlog** in the spec's order: Deploy (§15.1), Subgraphs (§15.2), Snapshots (§15.7), compiled blocks (§15.8), Presence view (§15.9), rig editor and Rive (§15.10), Convert panel (§15.5), library management screen, clickable prototype parity. | | |

Slices 7, 8 and 9 are independent of each other after 4 and can run in
parallel if there are hands for it. Nothing in 10 is worth starting
before 7 and 9 exist, because the Avatar's inputs come from them.

---

## 7. Open questions and assumptions

Things I decided so the plan could be written. Each is reversible now
and expensive later.

1. **Tauri over Electron. Settled in slice 2: it works.** The shell was
   built and then run twice, once in Chromium and once in a real Tauri
   window under WebKitGTK 2.52, and the two were compared pixel by pixel.
   WebKitGTK renders all of it with no fallbacks: CSS Modules, custom
   properties, `color-mix`, the bundled WOFF2 faces, the SVG wires, the
   dashed loop frame and the whole xyflow canvas. Over the canvas region
   0.79% of pixels differ, which is text antialiasing; the block and port
   geometry is identical. The inspector and library differ more (8.7% and
   4.7%) because line boxes round differently between the two font
   rasterisers, so panel text drifts by a few pixels down a long column.
   That is worth knowing for one reason: *pixel-matching an artboard can
   only ever be true on one engine*, and the engine that matters is
   WebKitGTK. What is **not** settled is performance: this was software
   rendering under Xvfb with no GPU, so frame times mean nothing yet.
   Measure that on real hardware before the canvas gets heavier.
2. **xyflow over a hand-rolled canvas.** Assumed because writing pan,
   zoom, drag, selection, minimap and hit-testing is weeks of work that
   xyflow has already done. The spec's geometry rules are all expressible
   as custom nodes and edges. If a later rule cannot be (I do not expect
   one), the abstraction is `packages/graph-core`, and the renderer is
   replaceable.
3. **No Tailwind.** Your brief's example stack had it. The token set is
   closed and small; CSS Modules plus custom properties keeps the
   mockups' inline-style discipline in a maintainable form. If you want
   Tailwind for velocity, it is a slice-0 decision with no downstream
   cost; say so.
4. **Engine as a separate process from day one.** More plumbing early
   (a socket, a client, process lifecycle) in exchange for Deploy being
   nearly free later and the shell surviving engine crashes. I think it
   is the right trade; it is the plan's biggest early cost.
5. **Rust owns the types; TypeScript is generated.** Assumed to keep the
   shell and engine from drifting on the `.loom` schema. The cost is a
   codegen step in the dev loop.
6. **Ollama is the only model provider in v1.** Remote providers are
   allowed by the Local-only switch (§15.4) but not built; the provider
   interface is one trait with one implementation until a second is
   wanted.
7. **Perception models are fixed choices**: YOLOv8n for objects,
   insightface for faces, whisper.cpp for speech, piper for voice, a
   small ONNX classifier for affect. Each is a block *setting* in the
   spec, so swapping is a runtime matter, but the v1 build tests one of
   each.
8. **Python is managed with `uv`** per workspace, and the block protocol
   is JSON lines over stdio. Signature parsing runs in Python (the `ast`
   module) so the engine does not reimplement the language.
9. **The four rigs are SVG state folders drawn from the mockups' `rigFace`
   renderer**, which already produces every expression as SVG. Idle
   behaviours (blink, breathe, drift) are procedural in the `avatar`
   crate, not authored per rig; a rig may override them in `rig.yaml`.
10. **Lip sync is amplitude-only in v1**: mouth openness from the speech
    port's RMS at 30 Hz. Viseme-level sync from the TTS engine's phoneme
    timings is a later improvement the port shape already allows.
11. **Secrets live in the OS keyring**, referenced by name from
    `.loom` (`env`), never by value, so graphs are safe to commit.
12. **Workspace = folder, library per workspace.** A block saved to the
    library lands in `<workspace>/blocks/`. A global library shared
    between workspaces is not planned for v1.
13. **The clickable prototype artboard is not a build target.** It
    predates the view rule and stays as a design artefact.
14. **Window minimum 1280 × 800.** Below the rail layout there is no
    design; the app refuses to shrink further rather than break.
15. **Wayland and X11 both, via Tauri's GTK backend.** Global hotkeys and
    always-on-top for the Avatar window behave differently under
    Wayland; the Avatar output target's *always on top* is best-effort
    there and I have not verified it.

Decided since this plan was written:

- **Name: Cyberloom** (SPEC §15.12). Graph files are `.loom`, the engine
  daemon is `loomd`, config lives in `~/.config/cyberloom/`, the
  application id is `dev.cyberloom.app`, and the URI scheme is
  `cyberloom://`. Free on crates.io and npm; three unrelated Cyberlooms
  exist on the web, two of them IT services firms, so the name is clear
  to use but will not own its search results.
- **Styling: CSS Modules, not Tailwind** (assumption 3 stands). Tailwind
  was in the brief only as an example. Its advantage is velocity for a
  team sharing one vocabulary, which is not this project; its cost here
  is that the canvas needs exact pixel geometry that reads badly as
  utility strings.
- **Model provisioning: the network is allowed on first run** (SPEC
  §15.13). Downloads are explicit, visible and weights-only; offline is a
  supported state rather than a failure; the per-graph Local only switch
  is unaffected and stays on by default. Slice 12 owns the first-run
  flow, so the installer stays small and the model choice is not frozen
  at build time.

Nothing is left blocking slice 0.

Corrected while building slice 1, from implementing the specification rather
than reading it:

- **A wire endpoint names a node, not a block.** A loop is a frame with ports
  of its own, and wires land on it. `Endpoint` says `node`.
- **The Loop needs an `item` output.** SPEC §6.8 omitted it; §13.2's wire table
  uses it, and a loop cannot pass the current item to the blocks inside the
  frame without one. Added to §6.8.
- **`tools`, `memory` and `exec` are closed types.** §4.1 said `any` "accepts
  every type" while each of those rows listed only its own kind. A handle is
  not a value and neither is a trigger, so the narrower reading wins; §4.1 now
  says so directly rather than leaving the two rows to contradict each other.
- **§4.1 still described the transform §15.5 removed.** Fixed to point at the
  Convert block.
- **The Data shelf was missing Convert**, which §15.5 added. Now in §6.7 and in
  the catalogue.
- **The built-in count is 49, not 44.** §6 lists 48 rows and §15.5 adds Convert.
- **§13.4's door_check example has a type error**: the block returns `Data` and
  Notify's `text` port takes `text`. The fixture inserts a Convert block, which
  is what §15.5 exists for.

Found while building slice 3, all of them in the seam between the store and
the canvas library rather than in the specification:

- **Autosave was costing an undo press.** The engine answers `graph.save` with
  its own canonical graph, which the store installs. That is a new object, so
  the history middleware recorded it as a step and one undo landed on the
  moment *after* the edit. `markSaved` now pauses the history around the swap.
  A save is not an edit.
- **Undo did not reach the file.** Time travel restores the graph and knows
  nothing about `dirty`, so an undone edit stayed on screen and never went
  back to disk. The keyboard map marks the document dirty after every undo
  and redo.
- **Shift-click did not multi-select.** xyflow's multi-select key is Ctrl on
  Linux and Cmd on macOS; SPEC §2.4 promises Shift. All three are accepted
  now, so the platform habit and the documented gesture both work.
- **The minimap drew nothing.** The nodes are rebuilt from the graph on every
  render, which threw away the size xyflow had measured, and the minimap draws
  from the node objects rather than from xyflow's own copy. The canvas keeps
  the measurements and hands them back.
- **The minimap was also being cropped.** xyflow reads its width and height off
  the inline `style` to work out its scale; given only CSS it drew a 200×150
  map inside a 138×88 box. That one size lives in the component, not in the
  stylesheet, and says why.

One gap left for slice 4, when settings start being read: **`SettingDef` has no
default.** The inspector shows an unset range at its minimum and an unset
choice as the first option, which reads as a value the user chose. Nothing runs
yet so nothing is wrong on disk — the file correctly says nothing — but the
panel should distinguish "unset" from "set to the bottom of the range", and it
cannot until the catalogue declares what each setting falls back to.

Found while building slice 4:

- **The play icon had been invisible since slice 0.** `Icon` hardcoded
  `fill="none"`, and `play` is a solid triangle drawn with no stroke — so the
  Run button had been showing its label and nothing else through three slices
  of screenshots. A stroke width of zero now means filled.
- **`SettingDef` still has no default** (carried over from slice 3), and it now
  matters: the runner falls back to the block's own setting where there is one
  and to nothing where there is not. Nothing guesses a temperature.
- **A block's title could be squeezed out of existence** by a chip appearing
  beside it. A truncated name is worse than no name only if there is one.
- **Live figures may not change a block's size.** Growing a running block
  nudges a graph whose layout was fine a second earlier; the figure hangs below
  the block instead, and the block comes forward so it is not drawn behind a
  neighbour. The z-index has to be set on the xyflow node, because every node
  is its own stacking context and a rule inside one cannot lift it.
- **`networkidle` is no longer a usable wait condition** in the screenshot
  tests: the event stream is a connection that stays open on purpose.

Two protocol details the generated TypeScript caught, both of which were
invisible from Rust:

- `rename_all` renames an enum's *variants*; `rename_all_fields` renames the
  fields inside them. Without the second, `tokens_in` was the one snake_case
  name in a camelCase protocol.
- Two types called `Value` cannot both be exported. Rust told them apart by
  module; TypeScript has one namespace for the whole schema.

And one thing this environment cannot prove: **the last mile of slice 4's
acceptance.** "Runs end to end against a local model" needs an Ollama, and the
network policy here denies both `ollama.com` and `registry.ollama.ai`. The
engine's side is covered against a scripted provider and against a stub server
speaking Ollama's wire format — the plan, the tool loop, the streaming, the
usage arithmetic, the event stream and the whole UI. What is untested is
whether a real model, handed these tool definitions, chooses to call them.
That is a question about the model rather than about this code, and it wants a
run on the CachyOS machine.

Found while building slice 5:

- **Double-clicking a block header did nothing.** SPEC §2.4 promises it cycles
  the views; only the toggle button did. Wired now, which is also how a custom
  block reaches its Code view.
- **A code change had to remember to re-parse.** The editors called `schedule`
  themselves, which made the reparse a thing every caller had to remember —
  and the first one that forgot (a paste, a format, an undo) would leave the
  block's interface quietly describing code that is no longer there. The
  source store subscribes to the document instead, so the change itself is the
  trigger.
- **The scratch file name collided.** Pid and block id are not enough: two runs
  of one graph, or two windows, overwrite each other's driver halfway through
  reading it.
- **Every input was coerced with `as_data`**, which reads text that happens to
  be JSON as the record it describes. Right for a `data` port, wrong
  everywhere else. The port's declared type decides the coercion now.
- **The screenshot scripts were writing to the real fixtures.** Autosave does
  exactly what it should; the scripts were pointed at the workspace under
  version control. They run against a copy now
  (`CYBERLOOM_WORKSPACE=/tmp/cl-ws`), which is worth keeping for every future
  run rather than relying on remembering to restore a file.

Two decisions the specification implies without stating, both recorded at the
code:

- **A float default between zero and one is a proportion**, and a slider is the
  right control for one — this is what makes §13.4's threshold a slider. A
  float outside that range is a quantity and gets a number field, because a
  slider needs bounds the code never gave.
- **The runtime provides the type names.** §10.1 writes annotations as `Image`,
  `Data`, `Text`; Python has never heard of them and evaluates annotations when
  the `def` runs, so a block written exactly as the specification writes it
  would fail on its first line. Providing them is what makes them the type
  system's names rather than a convention the parser happens to recognise.

**`SettingDef` now carries a default**, closing the gap slice 3 opened. The
rule, encoded in the catalogue's own tests: a default is declared when the
control *cannot* be unset — a switch has no third position, a segmented choice
always has something selected — or when the specification states one.
Everything else stays genuinely unset, and the inspector shows `unset` rather
than a number nobody chose. A temperature has none on purpose: the engine
leaves it out of the request and the provider's own default applies, which is
a better answer than one invented in a table here.

Half of it was not cosmetic. `flag` read `false` for any switch the file did
not mention, which made every safety switch off unless someone had turned it
on — the opposite of what SPEC §12.2 says about shell commands and physical
actions. A Terminal dropped on a canvas now warns before it runs, and a test
in the protocol suite caught the change by parking twenty of them on a
question nobody was answering.

Found while building slice 6:

- **Every Branch was classified as a tool nobody held.** The capability rule
  read "all wired outputs are closed types", and `exec` is closed — so a
  Branch, whose outputs are exec, was never scheduled at all. §4.3
  distinguishes handles from control flow: a capability is a block whose
  outputs are `tools` or `memory`, the things a holder *calls*. Control flow
  makes other blocks run, which is the opposite of waiting to be asked.
- **A file arriving the instant a folder armed was missed, silently and
  forever.** The listing of what was already there was taken inside the
  source's own thread, leaving a window after the block reported itself armed.
  It is taken before the thread starts now, so "armed" means it.
- **An `exec` wire seeded a null into the value map**, so a Terminal wired to a
  Schedule's tick ran an empty command — successfully, and to no effect.
- **A Terminal step that exits non-zero has failed.** It used to warn and carry
  on, which left continue-on-error with nothing to act on. Deliberately not how
  a Terminal behaves as a *tool*: there, exit 101 is a result the model reads,
  which is the whole of §13.1.

Two decisions worth recording:

- **One event at a time, and the overlap policy handles the rest.** A graph is
  a program with shared state — a Memory hub, a Terminal's working directory,
  a model's context — and running two events through it at once would make
  every one of those a race the user never wrote. Parallelism belongs inside a
  Loop frame, where §8.3 puts it and where the items are independent.
- **A warning inside a loop is asked once, before the loop starts**, naming how
  many items it covers. A prompt that appears two hundred times is not a
  prompt, and answering it two hundred times is not consent.

What slice 6 does **not** deliver: `notify` and `file-system` are not runnable
kinds, so inbox-triage runs live, keeps running, and reports those two blocks
per event rather than completing its archive and its Slack message. The live
machinery is complete and proven — sources arm, events fire only what is below
them, the loop repeats, the branch chooses, a hold holds and a stop releases
the port. The two leaf blocks belong to §6.2 and §6.9 and to their own slices.

Found while building slice 7:

- **A scratch folder named by pid and run id is not unique.** Two runs of one
  graph, a restarted run, or a test suite running in parallel all give two
  scratches one directory, and the first to finish deletes the other's frames
  out from under it. Same class as slice 5's driver collision, and the fix is
  the same: a process-wide counter in the name.
- **`f32` is the wrong width at a JSON boundary.** A detector produces `f32`
  and a confidence of 0.88 arrives in a panel as `0.8799999952316284`. The
  same lesson as slice 4's temperature, stated as a rule this time: the width
  is chosen for the boundary, not for the producer.
- **`box` is a Rust keyword and that is Rust's problem, not the protocol's.**
  A field called `box_` was leaking the workaround into the wire format.

Two decisions worth recording:

- **Capture goes through ffmpeg**, not through V4L2 and PipeWire directly. The
  alternative is unsafe ioctls, a format-negotiation dance per device and a
  different set of both per platform, to do badly what one subprocess already
  does well. It also gives a machine with no camera a camera: `lavfi:testsrc`
  is a real ffmpeg source, so a graph with a Webcam in it runs on a laptop
  with the lid shut, on a server, and in a test — which is the only reason any
  of slice 7 could be written here.
- **The privacy default is where the file is written.** SPEC §12.3 says frames
  and audio are not recorded unless the user turns recording on, so capture
  writes into a per-run folder that is deleted when the run ends, and `store`
  is what copies it somewhere durable. Nothing has to remember to clean up,
  because the folder going away takes everything with it.

What this environment cannot prove, and what it now can:

- **Capture is real.** ffmpeg is installed, frames and audio are really
  captured, and the resolution and sample rate that come back are checked with
  ffprobe. There is no camera here; `lavfi:` stands in for one, and the same
  code path opens `/dev/video0` on a machine that has one.
- **Perception is not.** `huggingface.co` and the GitHub release hosts are
  denied by the network policy, so there are no weights. Object detection,
  face recognition, speech and affect are a trait with a real
  ONNX-through-Python implementation and a scripted one; every path through
  the engine is pinned down, and whether yolo-v8n finds a door is not.

Found while building slice 7's panels:

- **A camera declared itself a source and nothing armed it.** `webcam` and
  `microphone` are `source: true` in the catalogue, so a graph holding one is
  live — and `arm()` had a comment saying devices "belong with the rest of the
  senses" and returned `None`. Door-watch went live, reported nothing armed,
  and sat there. Every capture in the engine had a test and not one of them was
  live, which is how a hole that size stayed invisible: the tests all ran the
  camera as a step. A device now ticks at its frame rate and the tick puts the
  block at the head of its own run, so the capture happens where every other
  capture happens — with the run's scratch folder, its recording rule and its
  preview.
- **A missing camera was a flood, not an error.** With the camera armed, a
  device that is not there fails on every tick: at 5 fps that is five identical
  lines a second, forever, and the console becomes useless at the moment it is
  most needed. SPEC §12.1 already says what to do — "a hardware fault *pauses*;
  one click resumes" — and it was written for exactly this. The engine now holds
  the graph itself, which needed a `held` event, because until now hold was
  always the person's and the shell only knew because it had asked.
- **One setting, two switches, one panel.** The Privacy section presents
  `store` as "Record to disk"; the generated Settings list presented the same
  setting as "Record frames" six rows below it. Which one is the real one is
  not a question a panel should raise.
- **A "store images" switch made an unconditional rule look like a default.**
  §12.3 does not say images are off by default, it says faces are stored as
  embeddings and never as images. Slice 1 read that as a switch defaulting off,
  so the panel offered a control directly under a sentence saying it could not
  happen. The switch is gone, the catalogue has the Match threshold the figure
  actually shows, and the test asserts the property — that no setting on that
  kind is about images — rather than a default.
- **A switch that cannot move is a lie unless it says so.** "Frames never leave
  this machine" is drawn as a switch in Figure 6 and is not a preference. It is
  drawn as one, disabled, and its hint opens with "Not a setting."
- **Prettier is not this project's formatter.** Running it on one file
  reformatted 300 lines it had no business touching. There is no config and no
  dependency; the house style is the style.

Still not proven here:

- **A custom block cannot call the perception models.** Door-watch's own code
  — the body from Figure 10 — calls `detect(frame)`, and the Python driver
  injects the type names but not the engine's `Perception`. The block fails
  with `NameError: name 'detect' is not defined`, which the engine reports
  correctly and carries on from, so the graph degrades the way it should. The
  fix is a callback channel from the driver back to the engine: the driver
  already speaks to the engine through a file, and this needs it to speak both
  ways mid-call. That is a slice of its own, with its own thinking about what a
  custom block is allowed to reach, rather than something to bolt onto the
  panels.

---

Found while building slice 8 (Memory):

- **A Memory hub is not a bundle, and the code thought it might be.** The
  binding resolver read `matches!(kind, "toolbox" | "memoryHub")` — and no kind
  is called `memoryHub`, so the branch never fired and the bug it would have
  caused never happened. It would have been a real one: a Toolbox is
  transparent because a model holding one calls `terminal.run` and never names
  the Toolbox, but a model holding a hub calls `remember()`, so dissolving the
  hub would have handed the model two stores and no way to choose between them.
  The line now names `toolbox` alone, with the reason written next to it.
- **`bindings` deliberately omits capabilities, and a hub needs its own slots.**
  A hub is a capability — its `memory` out is a handle — so it is not in
  `bindings` by design, and it still has to find the stores wired into it.
  `Plan` gained `slots`, which is the same computation without the filter.
- **Consolidation copies; it does not move.** Moving what falls out of working
  memory into the long-term store makes the two behaviours fight: a window is
  supposed to lose things, and if losing them is also how they become permanent
  then what survives depends on whether the graph happened to be busy. Worse,
  it means a Once run that learns one fact learns nothing permanently, because
  nothing ever overflowed. Copying separates them — working memory forgets on
  its own schedule, the long-term store keeps what mattered, and each episode is
  carried exactly once.
- **The same memory twice is one memory.** Long-term rows are keyed by a
  fingerprint of their text and kind rather than by a counter. A graph that runs
  every minute would otherwise write the same sentence sixty times an hour, and
  recall would return it twelve times.

Two decisions worth recording:

- **SQLite is bundled, not linked.** `rusqlite`'s `bundled` feature compiles
  SQLite from source into the binary. It costs a slow first build and it means
  the AppImage in slice 12 has no system dependency to detect, no version to be
  wrong about, and nothing to explain to a user whose distribution ships a
  different one.
- **Recall reads every vector.** No index. At the scale a person's assistant
  reaches — thousands of episodes — a scan is a few milliseconds and there is no
  index to keep correct; an index is the answer at a scale this program does not
  have, and a wrong index is worse than a scan. Written down so the day it stops
  being true is a decision rather than a discovery.

What this environment cannot prove:

- **Real embeddings.** Relevance is cosine similarity over whatever
  `Perception::embed` returns, and the scripted implementation returns a
  deterministic vector from the text's length. That proves the ordering, the
  cutoff, the round trip through SQLite and the difference between recall and
  recency; it does not prove that `nomic-embed-text` puts "who is Sam" near "Sam
  is the partner". The weights are not fetchable here (§15.13's download is
  denied by the network policy), and the seam between the two is one trait.

---

Found while building slice 9 (Actuators and feedback):

- **Capability and step are not opposites, and the rule said they were.** A
  block counted as a capability only when *every* wired output was a handle.
  §4.4's whole point is that a device which can be commanded can also report,
  and the Motors block carries all three shapes at once — `tool` is the handle,
  `state` streams telemetry, `fault` interrupts. So wiring up the telemetry
  turned the device into a step: the graph tried to *run* the motors, failed
  with "not a kind this engine can run yet", and the tool the orchestrator had
  been given was one nobody could call. The rule is now two rules. A block is a
  capability when *any* wired output is a handle; it is a step unless *every*
  wired output is a handle. A Terminal wired both ways is both, which is what
  §13.3 needs. A Branch is still a step, because `exec` is closed but is not a
  handle.
- **A device that is only ever called takes its turn by doing nothing.** With
  the rule fixed, Motors is still in the order — blocks reading its telemetry
  have to come after it — and there is nothing for it to do until somebody calls
  it. Reporting an error there would have been reporting that a servo failed to
  be asked a question.
- **A refused tool call left no trace.** The "not one of the tools you were
  given" path returned early, before the `tool.call` and `tool.result` events.
  The call a person most wants to see in the trace — the one that was stopped —
  was the one that left no record. Refusals now come out with every other
  outcome.

One decision worth recording:

- **A limit is the machine's geometry, not a policy about the user.** §12.1 says
  the application may warn and may not prevent, and a pan limit looks like a
  prevention. It is not: asking a servo for an angle it does not have is how a
  servo is broken, and a real controller refuses at its end stop. So a move past
  a limit does not happen and raises a fault, exactly as the hardware would —
  and the limits are two text fields the user owns.

What this environment cannot prove:

- **A real serial device.** `serialport` is in the build (without its libudev
  default feature, which will not compile here), and the `Serial` implementation
  writes a line and reads a line with a timeout. Nothing on this machine has a
  serial port, so what is proven is everything above the wire: the warning, the
  limits, the three feedback ports, the Toolbox that stops taking calls, the
  clearing call that starts it again. The seam is one trait with two
  implementations, as it is for models, perception and capture.

---

Found while building slice 10 (Avatar):

- **The step rule needed a second half.** Slice 9 made "a step is a block that
  is not entirely handles on the way out". An Avatar whose `tool` is held by a
  model and whose `express` is fed by an Affect block has only a handle out —
  and work arriving on a wire it has to act on. It was a capability and never
  ran, so the flow path §11.3 describes did not exist. The rule now looks at
  both directions: a block is excused from taking a turn only when everything
  it produces is a handle *and* nothing arrives on a wire. Either half alone is
  not enough.
- **A bundle in the order is a bundle that does nothing.** With the rule
  looking at inputs, a Toolbox with a `fault` wired into its `pause` became a
  step. That is right for ordering — the blocks reading it come after — and
  wrong to run, so `toolbox` and `memory-hub` join the devices that take an
  empty turn.
- **The Affect block already carried the answer, under a different name.** It
  writes `express` into its data precisely so an Avatar does not have to know
  where the threshold between a smile and a frown is (§11.2). The first draft of
  `run_face` looked for `expression`, `emotion` and `label` and would have
  silently ignored every Affect block in existence.
- **ffmpeg's `sine` is an eighth of full scale.** A test asserting "a sine wave
  should be loud" failed at 31 of 255 — and the envelope was right. The number
  worth asserting is that a tone and a silence do not look the same; the scaling
  itself is pinned separately against a WAV the test writes, so it is arithmetic
  rather than whatever amplitude ffmpeg felt like.
- **ffmpeg writes a LIST chunk before the data.** Reading a WAV by assuming the
  samples start at byte 44 would have worked on a file this project never
  produces. The envelope walks the RIFF chunks.

Two decisions worth recording:

- **Lip sync crosses the socket as a shape, not a sound.** The engine reads the
  speech audio it already has and sends twelve loudness buckets a second — a few
  hundred bytes rather than a few hundred kilobytes — and the shell animates the
  mouth from that. The audio itself never needs to reach the window for the
  mouth to move in time with it, which also means the mouth is right on a
  machine with no sound device.
- **Every rig names `#eyes` and `#mouth`.** That one convention is what lets a
  single animation drive four aesthetics: the blink, the gaze and the mouth are
  the same three transforms whether the face is two dots, a robot head, a glowing
  sphere or an 8 × 8 matrix. A rig without them still draws; it just holds still.

What this environment cannot prove:

- **A rig the user added.** The four reference rigs are bundled into the shell so
  a face never arrives late. Drawing a rig from the workspace needs the engine to
  hand its states over, which this slice did not open — the *vocabulary* is
  already generated from whatever rig the engine loaded, including a workspace
  one, so the model side works today and only the picture would be missing.
