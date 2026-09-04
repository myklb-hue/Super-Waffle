/**
 * The open graph, and everything that can be done to it.
 *
 * One store per open graph. The document is the graph exactly as the file holds
 * it, so what the canvas shows and what gets written are the same object rather
 * than two representations that have to be kept in step.
 *
 * Undo covers every canvas edit *including* view and size changes (SPEC §15.7),
 * which is why they go through the store rather than living in component state.
 * Selection and the viewport are deliberately outside the undone slice: undoing
 * should not move the camera or change what is selected, because neither is a
 * change to the graph.
 */

import { create } from "zustand";
import { temporal } from "zundo";
import {
  BLOCK_MIN_WIDTH,
  accepts,
  convertible,
  kind as lookupKind,
  snap,
  type Block,
  type Graph,
  type PortType,
  type Port,
  type Position,
  type SourceMode,
  type View,
  type Wire,
} from "@cyberloom/graph-core";

/** What a reparse changed (SPEC §10.3). */
export interface Reload {
  added: string[];
  removed: string[];
  retyped: string[];
  dropped: Wire[];
}

export type Selection =
  | { kind: "none" }
  | { kind: "block"; ids: string[] }
  | { kind: "wire"; id: string }
  | { kind: "frame"; id: string };

export interface DocumentState {
  /** The graph, as the file holds it. */
  graph: Graph;
  path: string;
  selection: Selection;
  /** What the engine last said is wrong with it. */
  problems: string[];
  /** Set on every edit, cleared when a save comes back. */
  dirty: boolean;

  load(path: string, graph: Graph, problems: string[]): void;
  markSaved(graph: Graph, problems: string[]): void;
  /** Mark the document changed without changing it. Undo and redo need this:
   *  they restore the graph through the time-travel middleware, which knows
   *  nothing about `dirty`, so without it an undone edit is never written. */
  touch(): void;

  select(selection: Selection): void;
  addToSelection(id: string): void;

  addBlock(block: Block): void;
  moveBlocks(ids: string[], dx: number, dy: number): void;
  setBlockView(id: string, view: View): void;
  resizeBlock(id: string, w: number, h: number | null): void;
  setSetting(id: string, name: string, value: unknown): void;
  renameBlock(id: string, title: string | null): void;
  /** Replace a custom block's inline code. */
  setCode(id: string, code: string): void;
  /** Point a custom block at a file instead, or back at inline code. */
  setSourceMode(id: string, mode: SourceMode, path?: string): void;
  /**
   * Apply a freshly parsed interface (SPEC §10.3).
   *
   * Ports that still exist keep their wires; a removed port drops its wire.
   * Both happen in one update, because a render between them would draw a
   * wire hanging off a port that is not there.
   */
  applyInterface(id: string, ports: Port[]): Reload;
  toggleDisabled(ids: string[]): void;

  connect(
    from: { node: string; port: string },
    to: { node: string; port: string },
  ): boolean;
  /**
   * Put a Convert between two ports whose types are compatible but not the
   * same (SPEC §15.5). Answers with the new block's id, or null.
   */
  convertOnWire(
    from: { node: string; port: string },
    to: { node: string; port: string },
  ): string | null;
  deleteSelection(): void;

  setGraphField<K extends keyof Graph>(key: K, value: Graph[K]): void;
}

/** Halfway between two blocks, which is where a block inserted on a wire goes. */
function between(
  graph: Graph,
  a: string,
  b: string,
): { x: number; y: number } {
  const one = graph.blocks.find((block) => block.id === a)?.position;
  const two = graph.blocks.find((block) => block.id === b)?.position;
  if (!one || !two) return { x: 0, y: 0 };
  return { x: Math.round((one.x + two.x) / 2), y: Math.round((one.y + two.y) / 2) };
}

/** A readable id: `llm`, then `llm-2`, `llm-3`. Ids end up in the file and in
 *  every wire, so they should be legible rather than random. */
export function freshId(graph: Graph, base: string): string {
  const taken = new Set([
    ...graph.blocks.map((b) => b.id),
    ...graph.frames.map((f) => f.id),
    ...graph.wires.map((w) => w.id),
  ]);
  if (!taken.has(base)) return base;
  for (let n = 2; ; n++) {
    const candidate = `${base}-${n}`;
    if (!taken.has(candidate)) return candidate;
  }
}

/** The type a port carries, wherever it is declared. */
export function portTypeOf(
  graph: Graph,
  node: string,
  port: string,
  side: "in" | "out",
): PortType | null {
  const block = graph.blocks.find((b) => b.id === node);
  if (block) {
    if (block.kind === "custom") {
      return (
        block.ports.find((p) => p.name === port && p.side === side)?.type ??
        null
      );
    }
    return (
      lookupKind(block.kind)?.ports.find(
        (p) => p.name === port && p.side === side,
      )?.type ?? null
    );
  }
  if (graph.frames.some((f) => f.id === node)) {
    return (
      lookupKind("loop")?.ports.find((p) => p.name === port && p.side === side)
        ?.type ?? null
    );
  }
  return null;
}

export const useDocument = create<DocumentState>()(
  temporal(
    (set, get) => ({
      graph: emptyGraph(),
      path: "",
      selection: { kind: "none" },
      problems: [],
      dirty: false,

      load(path, graph, problems) {
        set({
          path,
          graph,
          problems,
          dirty: false,
          selection: { kind: "none" },
        });
        // Opening a file is not an edit, so it must not be undoable — otherwise
        // the first Ctrl+Z after opening reverts to an empty document.
        useDocument.temporal.getState().clear();
      },

      markSaved(graph, problems) {
        // A save is not an edit. The engine returns the canonical graph, which
        // is a new object, and without pausing the history that would be
        // recorded as a step: every autosave would then cost an extra undo,
        // and one press after an edit would land on the moment just after it
        // rather than before.
        const history = useDocument.temporal.getState();
        history.pause();
        set({ graph, problems, dirty: false });
        history.resume();
      },

      touch() {
        set({ dirty: true });
      },

      select(selection) {
        set({ selection });
      },

      addToSelection(id) {
        const current = get().selection;
        const ids = current.kind === "block" ? current.ids : [];
        set({
          selection: {
            kind: "block",
            ids: ids.includes(id) ? ids.filter((x) => x !== id) : [...ids, id],
          },
        });
      },

      addBlock(block) {
        set((state) => ({
          graph: { ...state.graph, blocks: [...state.graph.blocks, block] },
          selection: { kind: "block", ids: [block.id] },
          dirty: true,
        }));
      },

      moveBlocks(ids, dx, dy) {
        set((state) => ({
          graph: mapBlocks(state.graph, ids, (block) => {
            const position = snap({
              x: block.position.x + dx,
              y: block.position.y + dy,
            });
            // A block dropped inside a loop frame joins it (SPEC §8.4).
            return {
              ...block,
              position,
              frame: frameContaining(state.graph, position),
            };
          }),
          dirty: true,
        }));
      },

      setBlockView(id, view) {
        set((state) => ({
          graph: mapBlocks(state.graph, [id], (block) => ({
            ...block,
            view,
            // Leaving a view that had a height drops it, so coming back to that
            // view starts from the default rather than a stale number.
            size:
              block.size && (view === "compact" || view === "summary")
                ? { w: block.size.w, h: null }
                : block.size,
          })),
          dirty: true,
        }));
      },

      resizeBlock(id, w, h) {
        set((state) => ({
          graph: mapBlocks(state.graph, [id], (block) => ({
            ...block,
            size: {
              w: Math.max(BLOCK_MIN_WIDTH, Math.round(w)),
              h: h ? Math.round(h) : null,
            },
          })),
          dirty: true,
        }));
      },

      setSetting(id, name, value) {
        set((state) => ({
          graph: mapBlocks(state.graph, [id], (block) => {
            const settings = { ...block.settings };
            // Clearing a setting removes it rather than writing an empty
            // string, so the file says nothing where the user chose nothing.
            if (value === "" || value === null || value === undefined) {
              delete settings[name];
            } else {
              settings[name] = value as never;
            }
            return { ...block, settings };
          }),
          dirty: true,
        }));
      },

      setCode(id, code) {
        set((state) => ({
          graph: mapBlocks(state.graph, [id], (block) => ({
            ...block,
            source: block.source
              ? { ...block.source, mode: 'inline', code }
              : { mode: 'inline', language: 'python', code, path: null },
          })),
          dirty: true,
        }));
      },

      setSourceMode(id, mode, path) {
        set((state) => ({
          graph: mapBlocks(state.graph, [id], (block) => ({
            ...block,
            source: block.source
              ? {
                  ...block.source,
                  mode,
                  // Switching to File keeps the code: someone who moves a
                  // block to a file and back should find their work still
                  // there rather than an empty editor.
                  path: mode === 'file' ? (path ?? block.source.path) : block.source.path,
                }
              : { mode, language: 'python', code: null, path: path ?? null },
          })),
          dirty: true,
        }));
      },

      applyInterface(id, ports) {
        const before = get().graph.blocks.find((b) => b.id === id)?.ports ?? [];
        const key = (p: Port) => `${p.side}.${p.name}`;
        const had = new Set(before.map(key));
        const has = new Set(ports.map(key));
        const added = ports.filter((p) => !had.has(key(p))).map((p) => p.name);
        const removed = before.filter((p) => !has.has(key(p))).map((p) => p.name);
        const retyped = ports
          .filter((p) => before.some((was) => key(was) === key(p) && was.type !== p.type))
          .map((p) => p.name);

        const surviving = new Set(ports.map((p) => p.name));
        const gone = (end: { node: string; port: string }) =>
          end.node === id && !surviving.has(end.port);
        const dropped = get().graph.wires.filter((w) => gone(w.from) || gone(w.to));

        if (added.length === 0 && removed.length === 0 && retyped.length === 0) {
          // Nothing changed, so nothing is written: a reparse that agrees with
          // what is already there must not mark the document dirty and set
          // autosave going for no reason.
          return { added, removed, retyped, dropped: [] };
        }

        set((state) => ({
          graph: {
            ...mapBlocks(state.graph, [id], (block) => ({ ...block, ports })),
            wires: state.graph.wires.filter((w) => !gone(w.from) && !gone(w.to)),
          },
          dirty: true,
        }));
        return { added, removed, retyped, dropped };
      },

      renameBlock(id, title) {
        set((state) => ({
          graph: mapBlocks(state.graph, [id], (block) => ({
            ...block,
            title: title && title.trim() ? title.trim() : null,
          })),
          dirty: true,
        }));
      },

      toggleDisabled(ids) {
        set((state) => {
          // All on, or all off: a mixed selection turns all of them off, which
          // is what makes a second press reverse the first.
          const disabled = state.graph.blocks.some(
            (b) => ids.includes(b.id) && !b.disabled,
          );
          return {
            graph: mapBlocks(state.graph, ids, (block) => ({
              ...block,
              disabled,
            })),
            dirty: true,
          };
        });
      },

      connect(from, to) {
        const graph = get().graph;
        const fromType = portTypeOf(graph, from.node, from.port, "out");
        const toType = portTypeOf(graph, to.node, to.port, "in");
        // The type grammar is the gate. This is the one place a wire can be
        // refused, and it refuses before the wire exists rather than adding a
        // broken one and reporting it (SPEC §4.1).
        if (!fromType || !toType || !accepts(fromType, toType)) return false;
        // The same pair twice is a no-op, not a second wire.
        if (
          graph.wires.some(
            (w) =>
              w.from.node === from.node &&
              w.from.port === from.port &&
              w.to.node === to.node &&
              w.to.port === to.port,
          )
        ) {
          return false;
        }
        const wire: Wire = {
          id: freshId(graph, `${from.node}-${to.node}`),
          from: { node: from.node, port: from.port },
          to: { node: to.node, port: to.port },
        };
        set((state) => ({
          graph: { ...state.graph, wires: [...state.graph.wires, wire] },
          dirty: true,
        }));
        return true;
      },

      convertOnWire(from, to) {
        const graph = get().graph;
        const fromType = portTypeOf(graph, from.node, from.port, "out");
        const toType = portTypeOf(graph, to.node, to.port, "in");
        if (!fromType || !toType || !convertible(fromType, toType)) return null;

        // "conversion is always a visible block" (SPEC §15.5). Not a property
        // of the wire, not a quiet coercion in the engine: a block on the
        // canvas that a person can select, read and delete.
        const id = freshId(graph, "convert");
        const midpoint = between(graph, from.node, to.node);
        const convert: Block = {
          id,
          kind: "convert",
          position: midpoint,
          view: "summary",
          settings: { to: toType },
          ports: [],
          disabled: false,
          breakpoint: false,
        };
        set((state) => ({
          graph: {
            ...state.graph,
            blocks: [...state.graph.blocks, convert],
            wires: [
              ...state.graph.wires,
              {
                id: freshId(state.graph, `${from.node}-${id}`),
                from: { node: from.node, port: from.port },
                to: { node: id, port: "value" },
              },
              {
                id: freshId({ ...state.graph, wires: [...state.graph.wires] }, `${id}-${to.node}`),
                from: { node: id, port: "value" },
                to: { node: to.node, port: to.port },
              },
            ],
          },
          dirty: true,
        }));
        return id;
      },

      deleteSelection() {
        const selection = get().selection;
        if (selection.kind === "none") return;
        set((state) => {
          let graph = state.graph;
          if (selection.kind === "wire") {
            graph = {
              ...graph,
              wires: graph.wires.filter((w) => w.id !== selection.id),
            };
          } else if (selection.kind === "block") {
            const ids = new Set(selection.ids);
            graph = {
              ...graph,
              blocks: graph.blocks.filter((b) => !ids.has(b.id)),
              // A wire with nothing at one end is not a wire.
              wires: graph.wires.filter(
                (w) => !ids.has(w.from.node) && !ids.has(w.to.node),
              ),
            };
          } else if (selection.kind === "frame") {
            graph = {
              ...graph,
              frames: graph.frames.filter((f) => f.id !== selection.id),
              wires: graph.wires.filter(
                (w) =>
                  w.from.node !== selection.id && w.to.node !== selection.id,
              ),
              // Deleting a frame frees its blocks; it does not delete them.
              blocks: graph.blocks.map((b) =>
                b.frame === selection.id ? { ...b, frame: null } : b,
              ),
            };
          }
          return { graph, selection: { kind: "none" }, dirty: true };
        });
      },

      setGraphField(key, value) {
        set((state) => ({
          graph: { ...state.graph, [key]: value },
          dirty: true,
        }));
      },
    }),
    {
      limit: 200,
      // Undo restores the graph and nothing else: not the selection, not the
      // camera. Both would be surprising, and neither is a change to the file.
      partialize: (state) => ({ graph: state.graph }),
      equality: (a, b) => a.graph === b.graph,
    },
  ),
);

/** Replace the named blocks, leaving the rest of the graph alone. */
function mapBlocks(
  graph: Graph,
  ids: string[],
  change: (block: Block) => Block,
): Graph {
  return {
    ...graph,
    blocks: graph.blocks.map((b) => (ids.includes(b.id) ? change(b) : b)),
  };
}

function frameContaining(graph: Graph, position: Position): string | null {
  for (const frame of graph.frames) {
    const { x, y } = frame.position;
    const w = frame.size.w;
    const h = frame.size.h ?? 0;
    if (
      position.x >= x &&
      position.y >= y &&
      position.x <= x + w &&
      position.y <= y + h
    ) {
      return frame.id;
    }
  }
  return null;
}

/** A graph with nothing in it, for the moment before the first file arrives. */
export function emptyGraph(): Graph {
  return {
    version: 1,
    id: "untitled",
    name: "Untitled",
    description: null,
    runMode: "once",
    localOnly: true,
    execution: { runtime: "local", concurrency: 4, timeoutSec: 120 },
    defaults: { provider: "ollama", model: "llama3.2:3b" },
    overlap: { policy: "queue", maxQueue: 100, coalesceMs: 0, loopParallel: 2 },
    between: { keepState: true, restartOnCrash: false },
    env: {},
    blocks: [],
    frames: [],
    wires: [],
    ui: { viewport: { x: 0, y: 0, zoom: 1 } },
  };
}

/** A new block of the given kind, dropped at a point on the canvas. */
export function blockOfKind(graph: Graph, kindId: string, at: Position): Block {
  const kind = lookupKind(kindId);
  const settings: Record<string, never> = {};
  return {
    id: freshId(graph, kindId),
    kind: kindId,
    title: null,
    position: snap(at),
    size: null,
    view: "summary",
    settings,
    ports: [],
    source:
      kindId === "custom"
        ? { mode: "inline", language: "python", code: null, path: null }
        : null,
    disabled: false,
    breakpoint: false,
    frame: frameContaining(graph, at),
  } satisfies Block & { kind: typeof kind extends undefined ? string : string };
}
