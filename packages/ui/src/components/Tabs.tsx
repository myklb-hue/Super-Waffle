import s from './ui.module.css';

export interface TabsProps {
  tabs: string[];
  active: string;
  onChange: (tab: string) => void;
}

/** The standard inspector tab strip. Every block panel reads
 *  Settings, Ports, Runs; sources read Settings, Ports, Events (SPEC 7.2). */
export function Tabs({ tabs, active, onChange }: TabsProps) {
  return (
    <div className={s.tabs} role="tablist">
      {tabs.map((t) => (
        <button
          key={t}
          type="button"
          role="tab"
          aria-selected={t === active}
          className={`${s.tab}${t === active ? ` ${s.tabActive}` : ''}`}
          onClick={() => onChange(t)}
        >
          {t}
        </button>
      ))}
    </div>
  );
}
