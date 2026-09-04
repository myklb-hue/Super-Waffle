import { Fragment } from 'react';
import s from './ui.module.css';

export interface KeyHintProps {
  /** A chord, written the way it is shown: "Cmd K", "Ctrl E", "Shift Drag". */
  keys: string;
}

/** Keycaps for a shortcut. Splits on spaces, so one string describes a chord. */
export function KeyHint({ keys }: KeyHintProps) {
  const parts = keys.split(/\s+/).filter(Boolean);
  return (
    <span className={s.keyHint}>
      {parts.map((k, i) => (
        <Fragment key={`${k}-${i}`}>
          {i > 0 && <span className={s.keySep}>+</span>}
          <kbd className={s.keycap}>{k}</kbd>
        </Fragment>
      ))}
    </span>
  );
}
