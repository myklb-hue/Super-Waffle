import type { CSSProperties } from 'react';
import { Label } from './Label';
import type { ColorToken } from '../types';
import s from './ui.module.css';

export interface SliderProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  unit?: string;
  /**
   * What to show instead of the number.
   *
   * A slider whose setting has never been chosen sits at its floor, and a
   * readout of `0` there reads as a deliberate zero rather than as nothing.
   * Passing `unset` says which it is; the handle still shows where a change
   * would start from.
   */
  display?: string;
  color?: ColorToken;
  disabled?: boolean;
  onChange: (value: number) => void;
}

/** A bounded numeric setting: a threshold, a temperature, a motor limit. */
export function Slider({
  label,
  value,
  min,
  max,
  step = 1,
  unit,
  display,
  color = 'accent',
  disabled = false,
  onChange,
}: SliderProps) {
  const pct = max === min ? 0 : ((value - min) / (max - min)) * 100;
  return (
    <span
      className={s.slider}
      style={{ '--c': `var(--${color})`, '--fill': `${pct}%` } as CSSProperties}
    >
      <span className={s.sliderHead}>
        <Label>{label}</Label>
        <span className={display ? s.sliderUnset : s.sliderValue}>
          {display ?? `${value}${unit ? ` ${unit}` : ''}`}
        </span>
      </span>
      <input
        className={s.sliderInput}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        disabled={disabled}
        aria-label={label}
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </span>
  );
}
