import type { CSSProperties } from 'react';
import type { ColorToken } from '../types';
import s from './ui.module.css';

export interface MeterProps {
  /** Values from 0 to 1, oldest first. */
  bars: number[];
  color?: ColorToken;
  label?: string;
}

/** A level over time: microphone amplitude, token rate, motor load. */
export function Meter({ bars, color = 'accent', label }: MeterProps) {
  return (
    <span
      className={s.meter}
      style={{ '--c': `var(--${color})` } as CSSProperties}
      role="img"
      aria-label={label}
    >
      {bars.map((b, i) => (
        <span
          key={i}
          className={s.meterBar}
          style={{ height: `${Math.max(0, Math.min(1, b)) * 100}%` }}
        />
      ))}
    </span>
  );
}
