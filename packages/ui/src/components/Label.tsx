import type { ReactNode } from 'react';
import s from './ui.module.css';

export interface LabelProps {
  children: ReactNode;
}

/** The uppercase mono label above a group of fields. */
export function Label({ children }: LabelProps) {
  return <span className={s.label}>{children}</span>;
}
