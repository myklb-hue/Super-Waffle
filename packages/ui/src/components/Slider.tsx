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
        <span className={s.sliderValue}>
          {value}
          {unit ? ` ${unit}` : ''}
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
