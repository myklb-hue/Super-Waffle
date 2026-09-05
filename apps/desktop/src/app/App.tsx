import { useEffect, useState } from 'react';
import { FaceWindow } from '../avatar/FaceWindow';
import { refreshRigs } from '../avatar/rigs';
import { Shell } from '../shell/Shell';
import { Picker, remember } from '../shell/Picker';
import { engineStatus, listWorkspace, useWorkspace } from '../stores/rpc';
import { useTabs } from '../stores/tabs';
import s from './App.module.css';

type State =
  | { phase: 'starting' }
  | { phase: 'ready'; detail: string }
  | { phase: 'unreachable'; message: string };

/**
 * Starts the engine, opens a graph and draws it.
 *
 * The app decides which graph opens first and nothing else: from there the tab
 * strip owns what is open and the document store owns what is being edited.
 */
export function App() {
  // `?face=<block>` is a window that is only that block's face (SPEC §11.5):
  // no canvas, no picker, no engine start of its own — it listens to the run
  // the main window is driving.
  const asked = new URLSearchParams(window.location.search);
  const faceOf = asked.get('face');
  if (faceOf) {
    return <FaceWindow block={faceOf} rig={asked.get('rig') ?? 'line'} />;
  }
  return <Main />;
}

function Main() {
  const [state, setState] = useState<State>({ phase: 'starting' });
  // The folder to serve, from the URL. Absent means "whatever the host started
  // with", which is how the application opens where it left off; `?pick`
  // forces the picker (SPEC §15.6).
  const [workspace, setWorkspace] = useState<string | null>(() => {
    const asked = new URLSearchParams(window.location.search);
    if (asked.has('pick')) return null;
    return asked.get('workspace') ?? 'default';
  });

  useEffect(() => {
    if (workspace === null) return;
    let cancelled = false;
    (async () => {
      try {
        if (workspace !== 'default') {
          await useWorkspace(workspace);
          remember(workspace);
        }
        const status = await engineStatus();
        const graphs = await listWorkspace();
        // The rigs this workspace can wear, including its own (SPEC §11.1).
        // Not awaited: the bundled four are drawn until the answer arrives.
        void refreshRigs();
        if (graphs.length === 0) {
          if (!cancelled) {
            setState({ phase: 'unreachable', message: 'the workspace holds no graphs' });
          }
          return;
        }
        // `?graph=` chooses which one opens first, and is how a screenshot
        // test reaches a fixture other than the first. Everything after that is
        // the tab strip's business (SPEC §15.6).
        const wanted = new URLSearchParams(window.location.search).get('graph');
        const chosen = graphs.find((g) => g.path.includes(wanted ?? '')) ?? graphs[0]!;
        await useTabs.getState().show(chosen.path);
        if (cancelled) return;
        setState({ phase: 'ready', detail: `${status.version} · ${status.graphs} graphs` });
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
  }, [workspace]);

  if (workspace === null) {
    return (
      <Picker
        onOpen={(path) => {
          setState({ phase: 'starting' });
          setWorkspace(path);
        }}
      />
    );
  }

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
        {/* A workspace that will not open is a reason to choose another one,
            not a dead end. */}
        <button type="button" className={s.waitingAction} onClick={() => setWorkspace(null)}>
          Open a different workspace
        </button>
      </div>
    );
  }

  return <Shell engine={{ state: 'ready', detail: state.detail }} />;
}
