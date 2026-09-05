/**
 * What is happening right now.
 *
 * The document store owns the graph, which is the program. This one owns the
 * *run*, which is not part of the program and never reaches the file: a `.loom`
 * file describes what to run and never a run. The two are deliberately
 * separate, and the separation is why an edit during a run cannot corrupt the
 * run's record of what happened, and why a run leaves nothing behind to undo.
 *
 * Everything here is derived from the engine's events. Nothing is inferred: if
 * the canvas shows a block as running it is because the engine said so, not
 * because the shell guessed from what it asked for.
 */

import { create } from 'zustand';
import type { BlockState, Decision, Graph, Level, PortValue, RunEvent } from '@cyberloom/graph-core';
import { answerWarning, pauseRun, startRun, stopRun, subscribeEvents } from './rpc';

/** How a block is doing, as the engine last reported it. */
export interface BlockRun {
  state: BlockState;
  /** The line shown inline while the run is in flight (SPEC §3.2). */
  figure: string | null;
  ms: number | null;
  /** Text streamed out of this block, as it arrives. */
  output: string;
  error: string | null;
}

/** What one face is wearing right now (SPEC §11.3). */
export interface FaceRun {
  rig: string;
  expression: string;
  intensity: number;
  /** The shape of the audio playing, 0–255 a bucket. Empty when it is quiet. */
  mouth: number[];
  gaze: string | null;
  /** Where that is, `0..1` from the top left, when the look came with a place. */
  gazeAt: [number, number] | null;
  /** The last one-shot gesture, and a count that changes every time one
   *  arrives — so the same gesture twice in a row plays twice. */
  gesture: string | null;
  gestureSeq: number;
  /** Asleep after the idle timeout (SPEC §11.4). */
  asleep: boolean;
  /** The idle the engine asked for: from the rig, overridden by the block. */
  blinkMs: number;
  breathePerMin: number;
  /** The mood's colour, for a Status light's swatch. */
  colour: string;
}

/** Something long being fetched — a model pulled into Ollama — as the engine
 *  last reported it (SPEC §15.13: downloads are explicit and visible). */
export interface ProgressRun {
  what: string;
  completed: number;
  total: number;
  status: string;
  done: boolean;
  error: string | null;
}

export interface ConsoleLine {
  /** Milliseconds since the run started, which is what the drawer shows. */
  at: number;
  source: string | null;
  level: Level;
  message: string;
}

/** One row of the Trace tab: who called what, and what came back. */
export interface TraceRow {
  at: number;
  caller: string;
  callee: string;
  name: string;
  arguments: unknown;
  result: string | null;
  ok: boolean | null;
  ms: number | null;
}

export interface Warning {
  id: string;
  block: string;
  action: string;
  reason: string;
  remember: boolean;
}

export interface Usage {
  tokensIn: number;
  tokensOut: number;
  rate: number;
  local: boolean;
}

export type Phase = 'idle' | 'running' | 'finished' | 'failed' | 'stopped';

/** Where a loop frame has got to (SPEC §3.5). */
export interface FrameRun {
  at: number;
  of: number;
  item: string | null;
}

interface RunStore {
  run: string | null;
  phase: Phase;
  /** `Date.now()` when the run started, so elapsed time is the clock's job and
   *  not a number this store has to keep re-rendering. */
  startedAt: number | null;
  ms: number | null;
  /** Every block the plan will visit, in order. */
  order: string[];
  blocks: Record<string, BlockRun>;
  /** What each source is watching, once it is armed (SPEC §8.2). */
  armed: Record<string, string>;
  /** The last thing each camera saw, as a data URI (Figure 6). */
  previews: Record<string, string>;
  /** What each face is wearing, by block id (SPEC §11.3). */
  faces: Record<string, FaceRun>;
  /** Pulls and downloads in flight or lately finished, by what they fetch. */
  progress: Record<string, ProgressRun>;
  frames: Record<string, FrameRun>;
  /** Whether the graph is holding: events queue, nothing runs (SPEC §8.1). */
  paused: boolean;
  /** Every event a live run has handled, newest first, for the graph panel's
   *  recent-events list. */
  recent: { at: number; source: string; detail: string }[];
  /** Wires that have carried a value. */
  active: string[];
  lines: ConsoleLine[];
  trace: TraceRow[];
  usage: Usage | null;
  /** The one warning the run is parked on, if any. */
  warning: Warning | null;
  results: PortValue[];
  /** What the orchestrator is writing, for the Run panel's live output. */
  live: string;
  problems: string[];
  /** Set when a run could not be started at all. */
  failure: string | null;

  apply: (event: RunEvent) => void;
  begin: (path: string, graph: Graph) => Promise<void>;
  halt: () => Promise<void>;
  hold: (paused: boolean) => Promise<void>;
  decide: (decision: Decision) => Promise<void>;
  clearConsole: () => void;
}

const EMPTY = {
  run: null,
  phase: 'idle' as Phase,
  startedAt: null,
  ms: null,
  order: [],
  blocks: {},
  armed: {},
  previews: {},
  faces: {},
  progress: {},
  frames: {},
  paused: false,
  recent: [],
  active: [],
  lines: [],
  trace: [],
  usage: null,
  warning: null,
  results: [],
  live: '',
  problems: [],
  failure: null,
};

/** How many console lines to keep.
 *
 *  A live graph can produce them faster than anyone reads them, and an
 *  unbounded array is a memory leak with a scrollbar. The drawer says when it
 *  has dropped some rather than quietly showing a partial history. */
export const CONSOLE_LIMIT = 2000;

export const useRun = create<RunStore>((set, get) => ({
  ...EMPTY,

  apply(event) {
    const since = () => {
      const started = get().startedAt;
      return started === null ? 0 : Date.now() - started;
    };
    const patch = (id: string, change: Partial<BlockRun>) =>
      set((s) => ({
        blocks: {
          ...s.blocks,
          [id]: {
            state: 'idle',
            figure: null,
            ms: null,
            output: '',
            error: null,
            ...s.blocks[id],
            ...change,
          },
        },
      }));
    const say = (line: ConsoleLine) =>
      set((s) => ({ lines: [...s.lines, line].slice(-CONSOLE_LIMIT) }));

    switch (event.event) {
      case 'run.started':
        set({
          ...EMPTY,
          progress: get().progress,
          run: event.data.run,
          phase: 'running',
          startedAt: Date.now(),
          order: event.data.order,
          problems: get().problems,
        });
        say({
          at: 0,
          source: null,
          level: 'info',
          message: `run started · ${event.data.order.length} steps`,
        });
        break;

      case 'block.state':
        patch(event.data.block, { state: event.data.state });
        break;

      case 'block.output':
        patch(event.data.block, {
          output: (get().blocks[event.data.block]?.output ?? '') + event.data.chunk,
        });
        set((s) => ({ live: s.live + event.data.chunk }));
        break;

      case 'block.done':
        patch(event.data.block, { figure: event.data.figure, ms: event.data.ms });
        break;

      case 'block.error':
        // No console line here. The engine emits one of its own for every
        // error, so writing a second from this event printed each failure
        // twice. The engine owns the log — a headless consumer reading only
        // `console` still sees everything — and the shell only renders it.
        patch(event.data.block, { error: event.data.message, state: 'error' });
        break;

      case 'block.preview':
        set((s) => ({
          previews: { ...s.previews, [event.data.block]: event.data.image },
        }));
        break;

      case 'source.armed':
        set((s) => ({ armed: { ...s.armed, [event.data.block]: event.data.state } }));
        break;

      case 'frame.state':
        set((s) => ({
          frames: {
            ...s.frames,
            [event.data.frame]: {
              at: event.data.at,
              of: event.data.of,
              item: event.data.item,
            },
          },
        }));
        // The start of a frame is the start of an event's work, which is what
        // the recent list is for.
        if (event.data.at === 0) {
          set((s) => ({
            recent: [
              { at: since(), source: event.data.frame, detail: `${event.data.of} items` },
              ...s.recent,
            ].slice(0, 20),
          }));
        }
        break;

      // The engine held the graph without being asked — a hardware fault
      // (SPEC §12.1). The transport reads `paused`, so this is all it takes for
      // the button to become Resume.
      // What the avatar's face is doing (SPEC §11.3). Kept per block, because
      // a graph may have more than one face in it — an Avatar and a Status
      // light are the same vocabulary in two media (§11.7).
      case 'face':
        set((st) => {
          const before = st.faces[event.data.block];
          const gestured = event.data.gesture !== null;
          return {
            faces: {
              ...st.faces,
              [event.data.block]: {
                rig: event.data.rig,
                expression: event.data.expression,
                intensity: event.data.intensity,
                mouth: event.data.mouth,
                gaze: event.data.gaze,
                gazeAt: event.data.gazeAt ? [event.data.gazeAt[0]!, event.data.gazeAt[1]!] : null,
                // A gesture is carried by one event and is not state; the
                // sequence number is how the face knows a new one arrived.
                gesture: gestured ? event.data.gesture : (before?.gesture ?? null),
                gestureSeq: (before?.gestureSeq ?? 0) + (gestured ? 1 : 0),
                asleep: event.data.asleep,
                blinkMs: event.data.blinkMs,
                breathePerMin: event.data.breathePerMin,
                colour: event.data.colour,
              },
            },
          };
        });
        break;

      case 'held':
        set({ paused: event.data.held });
        break;

      // A pull or a download is not a run's business and outlives `run.started`
      // resetting the rest, so it is kept on its own key rather than in EMPTY.
      case 'progress':
        set((s) => ({
          progress: {
            ...s.progress,
            [event.data.what]: {
              what: event.data.what,
              completed: event.data.completed,
              total: event.data.total,
              status: event.data.status,
              done: event.data.done,
              error: event.data.error,
            },
          },
        }));
        break;

      case 'wire.active':
        set((s) =>
          s.active.includes(event.data.wire) ? s : { active: [...s.active, event.data.wire] },
        );
        break;

      case 'console':
        if (event.data.message.includes(' → ') && event.data.source) {
          set((s) => ({
            recent: [
              {
                at: since(),
                source: event.data.source!,
                detail: event.data.message,
              },
              ...s.recent,
            ].slice(0, 20),
          }));
        }
        say({
          at: since(),
          source: event.data.source,
          level: event.data.level,
          message: event.data.message,
        });
        break;

      case 'tool.call':
        set((s) => ({
          trace: [
            ...s.trace,
            {
              at: since(),
              caller: event.data.caller,
              callee: event.data.callee,
              name: event.data.name,
              arguments: event.data.arguments,
              result: null,
              ok: null,
              ms: null,
            },
          ],
        }));
        break;

      case 'tool.result':
        set((s) => {
          // The last unanswered call to this tool is the one that just came
          // back. Matching on the name alone would answer the wrong row when a
          // model calls the same tool twice.
          const trace = [...s.trace];
          for (let i = trace.length - 1; i >= 0; i -= 1) {
            const row = trace[i]!;
            if (row.name === event.data.name && row.result === null) {
              trace[i] = {
                ...row,
                result: event.data.result,
                ok: event.data.ok,
                ms: event.data.ms,
              };
              break;
            }
          }
          return { trace };
        });
        break;

      case 'run.warning':
        set({ warning: { ...event.data } });
        say({
          at: since(),
          source: event.data.block,
          level: 'warn',
          message: event.data.action,
        });
        break;

      case 'run.usage':
        set({
          usage: {
            tokensIn: event.data.tokensIn,
            tokensOut: event.data.tokensOut,
            rate: event.data.rate,
            local: event.data.local,
          },
        });
        break;

      case 'run.finished':
        set({
          phase: event.data.outcome,
          ms: event.data.ms,
          results: event.data.results,
          warning: null,
        });
        say({
          at: event.data.ms,
          source: null,
          level: event.data.outcome === 'failed' ? 'error' : 'info',
          message: `run ${event.data.outcome} in ${(event.data.ms / 1000).toFixed(1)}s`,
        });
        break;
    }
  },

  async begin(path, graph) {
    // Clear before asking, so a second Run does not show the last run's blocks
    // while the engine is still deciding whether it can start.
    set({ ...EMPTY, phase: 'running', startedAt: Date.now() });
    try {
      const started = await startRun(path, graph);
      set({ run: started.run, problems: started.problems });
      for (const problem of started.problems) {
        set((s) => ({
          lines: [...s.lines, { at: 0, source: null, level: 'warn', message: problem }],
        }));
      }
    } catch (e) {
      // A run that could not start is shown, not thrown: the graph is still
      // there and the shell is still usable (SPEC §12.1).
      const message = e instanceof Error ? e.message : String(e);
      set((s) => ({
        phase: 'failed',
        failure: message,
        lines: [...s.lines, { at: 0, source: null, level: 'error', message }],
      }));
    }
  },

  async halt() {
    const { run } = get();
    await stopRun(run ?? undefined);
  },

  /** Hold the graph, or let it go (SPEC §8.1). */
  async hold(paused) {
    const { run } = get();
    set({ paused });
    await pauseRun(paused, run ?? undefined);
  },

  async decide(decision) {
    const { warning } = get();
    if (!warning) return;
    // Cleared straight away rather than when the engine confirms: the prompt
    // has been answered, and leaving it up while the answer travels invites a
    // second click on a question that is already settled.
    set({ warning: null });
    await answerWarning(warning.id, decision);
  },

  clearConsole() {
    set({ lines: [], trace: [] });
  },
}));

/** Subscribe the store to the engine, for the life of the window. */
export function listenToRuns(): () => void {
  return subscribeEvents((event) => useRun.getState().apply(event));
}

/** Milliseconds as `00:04.2`, the transport's clock (SPEC §8.1). */
export function elapsed(ms: number): string {
  const total = Math.max(0, ms) / 1000;
  const minutes = Math.floor(total / 60);
  const seconds = total - minutes * 60;
  return `${String(minutes).padStart(2, '0')}:${seconds.toFixed(1).padStart(4, '0')}`;
}
