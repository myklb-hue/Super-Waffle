import type { CSSProperties } from 'react';
import type { ColorToken } from '../types';
import s from './ui.module.css';

export interface SegmentedProps {
  options: string[];
  value: string;
  color?: ColorToken;
  disabled?: boolean;
  label?: string;
  onChange: (value: string) => void;
}

/** A small set of exclusive choices, all visible at once. */
export function Segmented({
  options,
  value,
  color = 'accent',
  disabled = false,
  label,
  onChange,
}: SegmentedProps) {
  return (
    <span
      className={s.segmented}
      role="radiogroup"
      aria-label={label}
      style={{ '--c': `var(--${color})` } as CSSProperties}
    >
      {options.map((o) => (
        <button
          key={o}
          type="button"
          role="radio"
          aria-checked={o === value}
          disabled={disabled}
          className={`${s.segmentedOption}${o === value ? ` ${s.segmentedActive}` : ''}`}
          onClick={() => onChange(o)}
        >
          {o}
        </button>
      ))}
    </span>
  );
}
