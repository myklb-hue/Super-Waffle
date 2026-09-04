import type { StatusState } from '../types';
import s from './ui.module.css';

const CLASS: Record<StatusState, string> = {
  idle: s.statusDotIdle!,
  queued: s.statusDotQueued!,
  running: s.statusDotRunning!,
  ok: s.statusDotOk!,
  error: s.statusDotError!,
  off: s.statusDotOff!,
  ready: s.statusDotReady!,
};

const TITLE: Record<StatusState, string> = {
  idle: 'Idle',
  queued: 'Queued',
  running: 'Running',
  ok: 'Finished',
  error: 'Error',
  off: 'Off',
  ready: 'Ready',
};

export interface StatusDotProps {
  state: StatusState;
}

/** What a block is doing, in seven pixels. The state is the whole component:
 *  there is no hover or disabled variant. */
export function StatusDot({ state }: StatusDotProps) {
  return (
    <span
      className={`${s.statusDot} ${CLASS[state]}`}
      role="img"
      aria-label={TITLE[state]}
      title={TITLE[state]}
    />
  );
}
