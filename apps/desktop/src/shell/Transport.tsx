import { useEffect, useState } from 'react';
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
  const running = phase === 'running';

  return (
    <div className={s.transport}>
      {running ? <Stop startedAt={startedAt} /> : <Run phase={phase} ms={finalMs} />}
    </div>
  );
}

function Run({ phase, ms }: { phase: string; ms: number | null }) {
  const path = useDocument((d) => d.path);
  const graph = useDocument((d) => d.graph);
  const begin = useRun((r) => r.begin);

  return (
    <>
      <button
        type="button"
        className={s.run}
        onClick={() => void begin(path, graph)}
        title="Run this graph once"
      >
        <Icon name="play" size={11} strokeWidth={0} />
        Run
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
 * The running transport: a clock and a stop square.
 *
 * The clock ticks here rather than in the store. Elapsed time is not something
 * the engine reports and not something worth an event ten times a second; it
 * is a function of one timestamp and the wall clock, so this is the only thing
 * that has to re-render for it.
 */
function Stop({ startedAt }: { startedAt: number | null }) {
  const halt = useRun((r) => r.halt);
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 100);
    return () => clearInterval(timer);
  }, []);

  return (
    <button type="button" className={s.running} onClick={() => void halt()} title="Stop the run">
      <span className={s.pulse} />
      running
      <span className={s.clock}>{elapsed(startedAt === null ? 0 : now - startedAt)}</span>
      <span className={s.square} />
    </button>
  );
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
