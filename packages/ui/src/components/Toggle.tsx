import type { CSSProperties } from 'react';
import type { ColorToken } from '../types';
import s from './ui.module.css';

export interface ToggleProps {
  on: boolean;
  color?: ColorToken;
  disabled?: boolean;
  label?: string;
  onChange: (on: boolean) => void;
}

/** A switch. Used bare only where the surrounding row says what it does;
 *  otherwise reach for SwitchRow. */
export function Toggle({ on, color = 'accent', disabled = false, label, onChange }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      disabled={disabled}
      className={`${s.toggle}${on ? ` ${s.toggleOn}` : ''}`}
      style={{ '--c': `var(--${color})` } as CSSProperties}
      onClick={() => onChange(!on)}
    >
      <span className={s.toggleKnob} />
    </button>
  );
}
