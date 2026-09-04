import type { ReactNode } from 'react';
import { Icon } from './Icon';
import type { IconName } from './icons';
import s from './ui.module.css';

export interface EmptyStateProps {
  icon: IconName;
  title: string;
  hint: string;
  actions?: ReactNode;
}

/** Nothing here yet, and what to do about it. This component is the empty
 *  state, so it has no empty state of its own. */
export function EmptyState({ icon, title, hint, actions }: EmptyStateProps) {
  return (
    <div className={s.emptyState}>
      <span className={s.emptyIcon}>
        <Icon name={icon} size={14} />
      </span>
      <div className={s.emptyTitle}>{title}</div>
      <div className={s.emptyHint}>{hint}</div>
      {actions && <div className={s.emptyActions}>{actions}</div>}
    </div>
  );
}
