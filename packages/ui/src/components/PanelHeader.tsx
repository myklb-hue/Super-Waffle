import type { CSSProperties } from 'react';
import { Icon } from './Icon';
import { Tabs } from './Tabs';
import type { IconName } from './icons';
import type { ColorToken } from '../types';
import s from './ui.module.css';

export interface PanelHeaderProps {
  icon: IconName;
  color: ColorToken;
  title: string;
  sub: string;
  tabs?: string[];
  active?: string;
  onTab?: (tab: string) => void;
  onMenu?: () => void;
}

/** The top of an inspector panel: what is selected, and what you can see
 *  about it. The panel below reshapes; this header does not. */
export function PanelHeader({
  icon,
  color,
  title,
  sub,
  tabs,
  active,
  onTab,
  onMenu,
}: PanelHeaderProps) {
  return (
    <header className={s.panelHeader} style={{ '--c': `var(--${color})` } as CSSProperties}>
      <div className={s.panelHeadRow}>
        <span className={s.panelIcon}>
          <Icon name={icon} size={14} color={color} strokeWidth={1.7} />
        </span>
        <span className={s.panelTitles}>
          <div className={s.panelTitle}>{title}</div>
          <div className={s.panelSub}>{sub}</div>
        </span>
        {onMenu && (
          <button type="button" className={s.panelMenu} aria-label="Panel menu" onClick={onMenu}>
            <Icon name="dots" size={14} />
          </button>
        )}
      </div>
      {tabs && active && onTab && <Tabs tabs={tabs} active={active} onChange={onTab} />}
    </header>
  );
}
