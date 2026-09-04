import { create } from 'zustand';
import type { Graph } from '@cyberloom/graph-core';
import { useDocument } from './document';
import { openGraph } from './rpc';

/**
 * The graphs that are open, as tabs across the top (SPEC §15.6).
 *
 * A workspace is a folder and open graphs are tabs in one window. This holds
 * the list and which one is in front; the document store still holds exactly
 * one graph, the one being edited.
 *
 * That is the compromise worth naming. Switching tabs saves the graph you are
 * leaving into this store and loads the one you are going to, so edits survive
 * a switch — but the undo history does not, because there is one time-travel
 * middleware and it belongs to the one document store. §15.7 wants undo per
 * graph and unlimited within a session; this gives per graph, and unlimited
 * until you switch away. Fixing it properly means a document store per tab,
 * which every component that calls `useDocument` would have to be handed
 * instead of importing — a change worth making deliberately rather than as a
 * side effect of adding tabs.
 */
export interface Tab {
  /** Workspace-relative, which is what the engine speaks. */
  path: string;
  name: string;
  /** The graph as this tab last had it, for a switch back. */
  graph: Graph;
  problems: string[];
}

interface Tabs {
  open: Tab[];
  /** The path of the tab in front, or null before anything is open. */
  active: string | null;
  /** Put a graph in front, opening it from the engine if it is not already up. */
  show(path: string): Promise<void>;
  close(path: string): void;
  /** Take the document's current state into the active tab. */
  keep(): void;
}

export const useTabs = create<Tabs>((set, get) => ({
  open: [],
  active: null,

  async show(path) {
    if (get().active === path) return;
    get().keep();

    // An open tab comes back from memory rather than from disk: it may hold
    // edits that have not been autosaved yet, and reading the file would
    // quietly throw them away.
    const already = get().open.find((t) => t.path === path);
    let tab: Tab;
    if (already) {
      tab = already;
    } else {
      const opened = await openGraph(path);
      tab = {
        path: opened.path,
        name: nameOf(opened.path, opened.graph),
        graph: opened.graph,
        problems: opened.problems,
      };
    }

    // Checked again inside `set`, not only before the await: two `show` calls
    // for the same path can both get past the first check while the engine is
    // still answering the first, and the tab would be added twice. React's
    // development double-invoke finds this immediately, which is what it is for.
    set((s) => ({
      open: s.open.some((t) => t.path === tab.path) ? s.open : [...s.open, tab],
      active: tab.path,
    }));
    useDocument.getState().load(tab.path, tab.graph, tab.problems);
  },

  close(path) {
    const { open, active } = get();
    if (active === path) get().keep();
    const left = open.filter((t) => t.path !== path);
    set({ open: left });
    if (active !== path) return;
    // Closing the front tab shows its neighbour, which is what every editor
    // does and what a person expects to be looking at afterwards.
    const at = open.findIndex((t) => t.path === path);
    const next: Tab | undefined = left[Math.min(at, left.length - 1)];
    if (next) {
      set({ active: null });
      void get().show(next.path);
    } else {
      set({ active: null });
    }
  },

  keep() {
    const { active } = get();
    if (!active) return;
    const doc = useDocument.getState();
    set((s) => ({
      open: s.open.map((t) =>
        t.path === active ? { ...t, graph: doc.graph, problems: doc.problems } : t,
      ),
    }));
  },
}));

/** What a tab is called: the graph's own name, falling back to its filename. */
export function nameOf(path: string, graph: Graph): string {
  return graph.name || (path.split('/').pop() ?? path).replace(/\.loom$/, '');
}
