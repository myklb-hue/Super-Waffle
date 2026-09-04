/**
 * Autosave.
 *
 * The graph is written 300 ms after the last edit (PLAN §4), so a drag or a
 * burst of typing produces one save rather than dozens. The engine reports
 * whether anything was actually written, and the shell adopts the canonical
 * form that came back — positions snapped, keys ordered — so what is on screen
 * and what is on disk never drift.
 *
 * A save that fails is *not* silent and *not* fatal: the top bar says "not
 * saved" and the status bar says why, and the next edit tries again.
 */

import { useEffect, useRef, useState } from 'react';
import { useDocument } from './document';
import { saveGraph } from './rpc';

const DELAY_MS = 300;

export type SaveState = 'saved' | 'saving' | 'dirty' | 'failed';

export function useAutosave(): { state: SaveState; error: string | null } {
  const dirty = useDocument((d) => d.dirty);
  const graph = useDocument((d) => d.graph);
  const path = useDocument((d) => d.path);
  const [state, setState] = useState<SaveState>('saved');
  const [error, setError] = useState<string | null>(null);
  const inFlight = useRef(false);

  useEffect(() => {
    if (!dirty || !path) return;
    setState('dirty');
    const timer = setTimeout(async () => {
      // One save at a time. An edit made during a save leaves `dirty` set, so
      // this effect runs again as soon as the reply lands.
      if (inFlight.current) return;
      inFlight.current = true;
      setState('saving');
      try {
        const saved = await saveGraph(path, graph);
        // Adopt what the engine wrote rather than what was sent.
        useDocument.getState().markSaved(saved.graph, saved.problems);
        setState('saved');
        setError(null);
      } catch (e) {
        setState('failed');
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        inFlight.current = false;
      }
    }, DELAY_MS);
    return () => clearTimeout(timer);
  }, [dirty, graph, path]);

  return { state, error };
}
