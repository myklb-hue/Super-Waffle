import type { CSSProperties } from 'react';
import type { ColorToken } from '../types';
import s from './ui.module.css';

export interface ChipProps {
  label: string;
  color: ColorToken;
  dot?: boolean;
  solid?: boolean;
  size?: 'sm' | 'md';
}

/** A small mono label carrying a colour: a language, a status, a run mode. */
export function Chip({ label, color, dot = false, solid = false, size = 'sm' }: ChipProps) {
  const cls = [s.chip, solid && s.chipSolid, size === 'md' && s.chipMd]
    .filter(Boolean)
    .join(' ');
  return (
    <span className={cls} style={{ '--c': `var(--${color})` } as CSSProperties}>
      {dot && <span className={s.chipDot} />}
      {label}
    </span>
  );
}
