/**
 * Keeping a custom block's interface in step with its code (SPEC §10.3).
 *
 * The code is re-parsed on save and when the editor loses focus. Typing is
 * neither, so this debounces: a parse on every keystroke would redraw the
 * ports while someone is halfway through a parameter name, and the ports
 * moving under the cursor is worse than a moment's lag.
 *
 * The interface is only *applied* when it parses. A file that does not is
 * shown red with its line number and the previous interface stays, so the
 * graph keeps running around it (SPEC §10.4) — which is why the last good
 * interface is remembered here rather than recomputed from the broken code.
 */

import { create } from 'zustand';
import type { Interface, Language, SourceError } from '@cyberloom/graph-core';
import { useDocument } from './document';
import { useRun } from './run';
import { reparse } from './rpc';

/** How long after the last keystroke to re-read the signature. */
export const REPARSE_AFTER_MS = 400;

export interface BlockSource {
  /** The last interface that parsed. Kept through a broken edit.  */
  interface: Interface | null;
  /** Every function the file declares, when it declares several. */
  siblings: Interface[];
  error: SourceError | null;
  /** When it last parsed, for the block's `updated 0.2 s ago`. */
  at: number | null;
  /** What the last reload changed, for the `+1 port` note. */
  note: string | null;
  parsing: boolean;
}

interface SourceStore {
  blocks: Record<string, BlockSource>;
  /** Re-read now, without waiting: a save, or the editor losing focus. */
  reload(id: string): Promise<void>;
  /** Re-read shortly, unless another keystroke arrives first. */
  schedule(id: string): void;
  forget(id: string): void;
}

const timers = new Map<string, ReturnType<typeof setTimeout>>();

export const useSource = create<SourceStore>((set, get) => ({
  blocks: {},

  schedule(id) {
    clearTimeout(timers.get(id));
    timers.set(
      id,
      setTimeout(() => {
        timers.delete(id);
        void get().reload(id);
      }, REPARSE_AFTER_MS),
    );
  },

  forget(id) {
    clearTimeout(timers.get(id));
    timers.delete(id);
    set((s) => {
      const blocks = { ...s.blocks };
      delete blocks[id];
      return { blocks };
    });
  },

  async reload(id) {
    const doc = useDocument.getState();
    const block = doc.graph.blocks.find((b) => b.id === id);
    const code = block?.source?.code;
    if (!block?.source || !code) return;

    const patch = (change: Partial<BlockSource>) =>
      set((s) => ({
        blocks: {
          ...s.blocks,
          [id]: {
            interface: null,
            siblings: [],
            error: null,
            at: null,
            note: null,
            parsing: false,
            ...s.blocks[id],
            ...change,
          },
        },
      }));

    patch({ parsing: true });
    try {
      const answer = await reparse(
        block.source.language as Language,
        code,
        block.title ?? undefined,
      );
      if (answer.error || !answer.chosen) {
        // The previous interface stays. That is the whole of §10.4: a block
        // goes red with a line number and the graph keeps running around it.
        patch({ error: answer.error, parsing: false });
        return;
      }
      const reload = useDocument.getState().applyInterface(id, answer.chosen.ports);
      const changes: string[] = [];
      if (reload.added.length) changes.push(`+${reload.added.length} port`);
      if (reload.removed.length) changes.push(`−${reload.removed.length} port`);

      // A dropped wire says so in the console, beside everything else that
      // happened to the graph.
      for (const wire of reload.dropped) {
        useRun.setState((r) => ({
          lines: [
            ...r.lines,
            {
              at: 0,
              source: id,
              level: 'warn',
              message: `dropped ${wire.from.node}.${wire.from.port} → ${wire.to.node}.${wire.to.port}: the port it landed on no longer exists`,
            },
          ],
        }));
      }

      patch({
        interface: answer.chosen,
        siblings: answer.blocks,
        error: null,
        at: Date.now(),
        note: changes.length ? changes.join(' · ') : null,
        parsing: false,
      });
    } catch {
      // The engine is unreachable. The block keeps what it had rather than
      // losing its interface because a socket blinked.
      patch({ parsing: false });
    }
  },
}));

/**
 * Watch the document, so a code change always re-parses.
 *
 * The editors could call `schedule` themselves — and did — but that made the
 * reparse a thing every caller had to remember, and the first caller that
 * forgot (a paste, a format, an undo) left the block's interface quietly
 * describing code that is no longer there. Subscribing to the document
 * instead makes the change itself the trigger, which is the only version that
 * cannot be forgotten.
 */
export function watchDocument(): () => void {
  let previous = codeOf(useDocument.getState().graph);
  return useDocument.subscribe((state) => {
    const now = codeOf(state.graph);
    for (const [id, code] of now) {
      if (previous.get(id) !== code) useSource.getState().schedule(id);
    }
    // A block that is gone should not keep a timer alive against it.
    for (const id of previous.keys()) {
      if (!now.has(id)) useSource.getState().forget(id);
    }
    previous = now;
  });
}

function codeOf(graph: { blocks: { id: string; kind: string; source?: { code?: string | null } | null }[] }) {
  const out = new Map<string, string>();
  for (const block of graph.blocks) {
    if (block.kind === 'custom') out.set(block.id, block.source?.code ?? '');
  }
  return out;
}

/** `0.2 s ago`, the note the block and the inspector both show. */
export function ago(at: number | null, now: number = Date.now()): string {
  if (at === null) return 'not yet';
  const seconds = Math.max(0, now - at) / 1000;
  if (seconds < 60) return `${seconds.toFixed(1)} s ago`;
  if (seconds < 3600) return `${Math.round(seconds / 60)} min ago`;
  return `${Math.round(seconds / 3600)} h ago`;
}
