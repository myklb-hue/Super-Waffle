import { useEffect, useState } from 'react';
import type { Graph } from '@cyberloom/graph-core';
import { Shell } from '../shell/Shell';
import { engineStatus, listWorkspace, openGraph } from '../stores/rpc';
import s from './App.module.css';

type State =
  | { phase: 'starting' }
  | { phase: 'ready'; graph: Graph; path: string; problems: string[]; detail: string }
  | { phase: 'unreachable'; message: string };

/**
 * Opens the first graph in the workspace and draws it.
 *
 * There is no workspace picker and no tabs yet: those are slice 11. What this
 * proves is the whole path — engine process, socket, format, catalogue,
 * geometry, renderer — with a real file at one end and pixels at the other.
 */
export function App() {
  const [state, setState] = useState<State>({ phase: 'starting' });

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const status = await engineStatus();
        const graphs = await listWorkspace();
        if (graphs.length === 0) {
          if (!cancelled) {
            setState({ phase: 'unreachable', message: 'the workspace holds no graphs' });
          }
          return;
        }
        // Until the workspace picker and tabs exist (slice 11), `?graph=`
        // chooses which one to open. It is also how the screenshot tests reach
        // a fixture other than the first.
        const wanted = new URLSearchParams(window.location.search).get('graph');
        const chosen = graphs.find((g) => g.path.includes(wanted ?? '')) ?? graphs[0]!;
        const open = await openGraph(chosen.path);
        if (cancelled) return;
        setState({
          phase: 'ready',
          graph: open.graph,
          path: open.path,
          problems: open.problems,
          detail: `${status.version} · ${status.graphs} graphs`,
        });
      } catch (error) {
        if (!cancelled) {
          setState({
            phase: 'unreachable',
            message: error instanceof Error ? error.message : String(error),
          });
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (state.phase === 'starting') {
    return <div className={s.waiting}>Starting the engine…</div>;
  }

  // The engine being down is a state the shell draws, not a crash. It is the
  // normal consequence of the two being separate processes.
  if (state.phase === 'unreachable') {
    return (
      <div className={s.waiting}>
        <div className={s.waitingTitle}>The engine is not answering</div>
        <div className={s.waitingDetail}>{state.message}</div>
      </div>
    );
  }

  return (
    <Shell
      graph={state.graph}
      path={state.path}
      problems={state.problems}
      engine={{ state: 'ready', detail: state.detail }}
    />
  );
}
