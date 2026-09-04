import { Icon } from './Icon';
import type { IconName } from './icons';
import type { ThirdView, View } from '../types';
import s from './ui.module.css';

export interface ViewToggleProps {
  active: View;
  /** A block with no third view gets a two-position toggle (SPEC 3.4). */
  third?: ThirdView;
  onChange: (view: View) => void;
}

const ICON: Record<View, IconName> = {
  compact: 'minus',
  summary: 'form',
  code: 'code',
  stage: 'fit',
};

const TITLE: Record<View, string> = {
  compact: 'Compact',
  summary: 'Summary',
  code: 'Code',
  stage: 'Stage',
};

/** How much of a block is drawn. Shown on hover or selection, remembered per
 *  block per graph. */
export function ViewToggle({ active, third = null, onChange }: ViewToggleProps) {
  const views: View[] = third ? ['compact', 'summary', third] : ['compact', 'summary'];
  return (
    <span className={s.viewToggle} role="radiogroup" aria-label="Block view">
      {views.map((v) => (
        <button
          key={v}
          type="button"
          role="radio"
          aria-checked={v === active}
          aria-label={TITLE[v]}
          title={TITLE[v]}
          className={`${s.viewOption}${v === active ? ` ${s.viewOptionActive}` : ''}`}
          onClick={() => onChange(v)}
        >
          <Icon name={ICON[v]} size={10} strokeWidth={2} />
        </button>
      ))}
    </span>
  );
}
