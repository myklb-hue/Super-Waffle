import { useEffect, useState } from 'react';
import type { RunMode } from '@cyberloom/graph-core';
import { Chip, Icon } from '@cyberloom/ui';
import { useDocument } from '../stores/document';
import { elapsed, useRun } from '../stores/run';
import s from './Transport.module.css';

/**
 * The Run button, and what it becomes while a graph is running (SPEC §8.1).
 *
 * Once mode only in this slice: Live and Schedule need sources, which arrive
 * with live graphs. The control is deliberately the same one in both states
 * rather than a Run button that grows a Stop button beside it — there is one
 * transport, and it reads as one thing.
 */
export function Transport() {
  const phase = useRun((r) => r.phase);
  const startedAt = useRun((r) => r.startedAt);
  const finalMs = useRun((r) => r.ms);
  const paused = useRun((r) => r.paused);
  const mode = useDocument((d) => d.graph.runMode);
  const running = phase === 'running';

  return (
    <div className={s.transport}>
      {running ? (
        <Live mode={mode} paused={paused} startedAt={startedAt} />
      ) : (
        <Run phase={phase} ms={finalMs} mode={mode} />
      )}
    </div>
  );
}

/**
 * The transport at rest, and what the last run came to.
 *
 * The label follows the mode, because pressing it does different things: a
 * Once graph runs and stops, a live one arms its sources and stays up.
 */
function Run({ phase, ms, mode }: { phase: string; ms: number | null; mode: RunMode }) {
  const path = useDocument((d) => d.path);
  const graph = useDocument((d) => d.graph);
  const begin = useRun((r) => r.begin);

  return (
    <>
      <button
        type="button"
        className={s.run}
        onClick={() => void begin(path, graph)}
        title={mode === 'once' ? 'Run this graph once' : 'Arm this graph and leave it running'}
      >
        <Icon name="play" size={11} strokeWidth={0} />
        {mode === 'once' ? 'Run' : mode === 'live' ? 'Go live' : 'Arm'}
      </button>
      {ms !== null && (
        <span className={phase === 'failed' ? s.lastFailed : s.last}>
          {phase} in {(ms / 1000).toFixed(1)}s
        </span>
      )}
    </>
  );
}

/**
 * The transport while a graph is going (SPEC §8.1, Figure 14).
 *
 * Four states, one control. Once counts up and stops; Live counts up and does
 * not; Schedule sleeps between ticks and says so; Paused keeps queueing and
 * runs nothing. The colour is the difference the eye reads first — green for
 * working, amber for waiting, grey for held.
 */
function Live({
  mode,
  paused,
  startedAt,
}: {
  mode: RunMode;
  paused: boolean;
  startedAt: number | null;
}) {
  const halt = useRun((r) => r.halt);
  const hold = useRun((r) => r.hold);
  const queued = useRun((r) => r.recent.length);
  const [now, setNow] = useState(Date.now());

  // The clock ticks here rather than in the store. Elapsed time is not
  // something the engine reports and not worth an event ten times a second;
  // it is one timestamp and the wall clock, so this is the only thing that
  // re-renders for it.
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 100);
    return () => clearInterval(timer);
  }, []);

  const since = startedAt === null ? 0 : now - startedAt;
  const tone = paused ? s.held : mode === 'schedule' ? s.waiting : s.going;
  const label = paused
    ? `paused · queue ${queued}`
    : mode === 'once'
      ? `running ${elapsed(since)}`
      : mode === 'schedule'
        ? `scheduled ${longer(since)}`
        : `live ${longer(since)}`;

  return (
    <span className={`${s.running} ${tone}`}>
      <span className={paused ? s.stillDot : s.pulse} />
      <span className={s.clock}>{label}</span>
      {/* Only a graph that keeps going can be held: pausing a Once run would
          be pausing something that is already nearly over. */}
      {mode !== 'once' && (
        <button
          type="button"
          className={s.pill}
          onClick={() => void hold(!paused)}
          title={paused ? 'Resume' : 'Hold: events keep queueing'}
        >
          <Icon name={paused ? 'play' : 'stop'} size={10} strokeWidth={0} />
        </button>
      )}
      <button type="button" className={s.pill} onClick={() => void halt()} title="Stop the run">
        <span className={s.square} />
      </button>
    </span>
  );
}

/** `4h 12m`, `12m`, `4.2s` — a live graph is up for hours, so the clock reads
 *  at the scale it has reached rather than counting seconds all day. */
function longer(ms: number): string {
  const total = Math.max(0, ms) / 1000;
  if (total < 60) return `${total.toFixed(1)}s`;
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total - hours * 3600) / 60);
  return hours > 0 ? `${hours}h ${minutes}m` : `${minutes}m`;
}

/** The run's figures along the bottom right, while there are any. */
export function RunFigures() {
  const usage = useRun((r) => r.usage);
  const phase = useRun((r) => r.phase);
  const blocks = useRun((r) => r.blocks);
  if (phase === 'idle') return null;
  const errors = Object.values(blocks).filter((b) => b.state === 'error').length;
  return (
    <span className={s.figures}>
      {usage && (
        <>
          <span>{usage.tokensOut} tok</span>
          {usage.rate > 0 && <span>{usage.rate.toFixed(0)} tok/s</span>}
        </>
      )}
      <span className={errors > 0 ? s.errors : undefined}>
        {errors} error{errors === 1 ? '' : 's'}
      </span>
    </span>
  );
}

/**
 * The warning prompt (SPEC §12.1).
 *
 * It describes the action and offers a Continue. There is no button here that
 * refuses the action while letting the graph carry on, because the application
 * may warn before a dangerous action and may not prevent one: the choice is
 * between going ahead and stopping the run.
 */
export function WarningPrompt() {
  const warning = useRun((r) => r.warning);
  const decide = useRun((r) => r.decide);
  if (!warning) return null;

  return (
    <div className={s.scrim} role="dialog" aria-modal="true" aria-labelledby="warning-action">
      <div className={s.prompt}>
        <header className={s.promptHead}>
          <Icon name="shield" size={14} color="warn" />
          <span>{warning.block}</span>
          <Chip label="waiting" color="warn" dot />
        </header>
        <p className={s.action} id="warning-action">
          {warning.action}
        </p>
        <p className={s.reason}>{warning.reason}</p>
        <footer className={s.promptFoot}>
          <button type="button" className={s.ghost} onClick={() => void decide('stop')}>
            Stop run
          </button>
          {warning.remember && (
            <button
              type="button"
              className={s.ghost}
              onClick={() => void decide('continueAlways')}
              title="Do not ask again for this block during this run"
            >
              Don't warn again
            </button>
          )}
          <button type="button" className={s.primary} onClick={() => void decide('continue')}>
            Continue
          </button>
        </footer>
      </div>
    </div>
  );
}
