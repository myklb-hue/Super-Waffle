import type { CSSProperties, ReactNode } from 'react';
import { Label } from './Label';
import type { ColorToken } from '../types';
import s from './ui.module.css';

export interface SectionProps {
  title: string;
  /** Tints the rule and the title: --err for safety and privacy groups. */
  tint?: ColorToken;
  right?: ReactNode;
  children: ReactNode;
}

/** One labelled group inside an inspector panel. */
export function Section({ title, tint, right, children }: SectionProps) {
  return (
    <section
      className={`${s.section}${tint ? ` ${s.sectionTinted}` : ''}`}
      style={tint ? ({ '--c': `var(--${tint})` } as CSSProperties) : undefined}
    >
      <header className={s.sectionHead}>
        <Label>{title}</Label>
        {right}
      </header>
      {children}
    </section>
  );
}
