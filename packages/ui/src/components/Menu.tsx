import { Icon } from './Icon';
import { KeyHint } from './KeyHint';
import type { IconName } from './icons';
import s from './ui.module.css';

export interface MenuItem {
  id: string;
  label: string;
  icon?: IconName;
  keys?: string;
  danger?: boolean;
  disabled?: boolean;
  /** Draws a rule above this item. */
  separated?: boolean;
}

export interface MenuProps {
  items: MenuItem[];
  onSelect: (id: string) => void;
}

/** The overflow menu on a block or a panel. Items carry their shortcut, so the
 *  menu teaches the keyboard rather than replacing it. */
export function Menu({ items, onSelect }: MenuProps) {
  return (
    <div className={s.menu} role="menu">
      {items.map((item) => (
        <div key={item.id}>
          {item.separated && <div className={s.menuSep} role="separator" />}
          <button
            type="button"
            role="menuitem"
            disabled={item.disabled}
            className={`${s.menuItem}${item.danger ? ` ${s.menuItemDanger}` : ''}`}
            onClick={() => onSelect(item.id)}
          >
            {item.icon && <Icon name={item.icon} size={12} />}
            <span className={s.menuLabel}>{item.label}</span>
            {item.keys && <KeyHint keys={item.keys} />}
          </button>
        </div>
      ))}
    </div>
  );
}
