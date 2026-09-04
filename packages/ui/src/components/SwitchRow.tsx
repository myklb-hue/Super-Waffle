import { Toggle } from './Toggle';
import type { ColorToken } from '../types';
import s from './ui.module.css';

export interface SwitchRowProps {
  label: string;
  hint?: string;
  on: boolean;
  color?: ColorToken;
  disabled?: boolean;
  onChange: (on: boolean) => void;
}

/** A switch with its explanation. The hint is where a warning setting says
 *  what it will and will not stop (SPEC 12: warn, never block). */
export function SwitchRow({
  label,
  hint,
  on,
  color = 'accent',
  disabled = false,
  onChange,
}: SwitchRowProps) {
  return (
    <span className={s.switchRow}>
      <span className={s.switchRowText}>
        <span className={s.switchRowLabel}>{label}</span>
        {hint && <span className={s.switchRowHint}>{hint}</span>}
      </span>
      <Toggle on={on} color={color} disabled={disabled} label={label} onChange={onChange} />
    </span>
  );
}
