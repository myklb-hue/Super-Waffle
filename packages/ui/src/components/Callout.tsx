import type { CSSProperties } from 'react';
import type { ColorToken } from '../types';
import s from './ui.module.css';

export interface CalloutProps {
  title: string;
  body: string;
  color?: ColorToken;
  /** Dashed and unfilled: a hint about something absent rather than a notice
   *  about something present. */
  dashed?: boolean;
}

/** A short notice inside a panel. A warning uses --warn, a safety or privacy
 *  boundary uses --err; neither ever blocks the user (SPEC 12). */
export function Callout({ title, body, color = 'accent', dashed = false }: CalloutProps) {
  return (
    <div
      className={`${s.callout}${dashed ? ` ${s.calloutDashed}` : ''}`}
      style={{ '--c': `var(--${color})` } as CSSProperties}
    >
      <div className={s.calloutTitle}>{title}</div>
      <div className={s.calloutBody}>{body}</div>
    </div>
  );
}

export type DashedHintProps = Omit<CalloutProps, 'dashed'>;

/** A Callout that describes what is not there yet. */
export function DashedHint(props: DashedHintProps) {
  return <Callout {...props} dashed />;
}
