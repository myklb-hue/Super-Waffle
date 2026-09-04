import type { CSSProperties } from 'react';
import { Icon } from './Icon';
import { StatusDot } from './StatusDot';
import type { IconName } from './icons';
import type { PortType, StatusState } from '../types';
import s from './ui.module.css';

export interface ConnectionRowProps {
  icon: IconName;
  name: string;
  meta: string;
  kind: PortType;
  /** 'pending' is a wire that has been drawn but not yet accepted. */
  state: StatusState | 'pending';
}

/** What a block is wired to: one row per connection in the Ports tab. */
export function ConnectionRow({ icon, name, meta, kind, state }: ConnectionRowProps) {
  return (
    <span className={s.connRow} style={{ '--c': `var(--type-${kind})` } as CSSProperties}>
      <span className={s.connIcon}>
        <Icon name={icon} size={12} color={`type-${kind}`} strokeWidth={1.7} />
      </span>
      <span className={s.connText}>
        <span className={s.connName}>{name}</span>
        <span className={s.connMeta}>{meta}</span>
      </span>
      {state === 'pending' ? (
        <span className={s.connPending} role="img" aria-label="Pending" title="Pending" />
      ) : (
        <StatusDot state={state} />
      )}
    </span>
  );
}
