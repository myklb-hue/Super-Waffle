import { useState, type ReactNode } from 'react';
import s from './ui.module.css';

export interface TooltipProps {
  content: ReactNode;
  children: ReactNode;
}

/** Hover help. Shown on focus as well as hover, so it is reachable from the
 *  keyboard. */
export function Tooltip({ content, children }: TooltipProps) {
  const [open, setOpen] = useState(false);
  return (
    <span
      className={s.tooltipWrap}
      onPointerEnter={() => setOpen(true)}
      onPointerLeave={() => setOpen(false)}
      onFocus={() => setOpen(true)}
      onBlur={() => setOpen(false)}
    >
      {children}
      {open && <span className={s.tooltip} role="tooltip">{content}</span>}
    </span>
  );
}
