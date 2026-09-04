import { useEffect, useState } from 'react';
import { Icon } from '@cyberloom/ui';
import { listWorkspace } from '../stores/rpc';
import { useDocument } from '../stores/document';
import { useTabs } from '../stores/tabs';
import s from './Tabs.module.css';

/**
 * The graphs that are open, across the top of the canvas (SPEC §15.6).
 *
 * A workspace is a folder and its graphs are tabs in one window. The `+` opens
 * one that is not open yet, which is the only way into the rest of the folder
 * without a file dialog — and a file dialog belongs to the workspace picker
 * rather than here.
 */
export function Tabs() {
  const open = useTabs((t) => t.open);
  const active = useTabs((t) => t.active);
  const show = useTabs((t) => t.show);
  const close = useTabs((t) => t.close);
  const dirtyPath = useDocument((d) => d.path);
  const [choosing, setChoosing] = useState(false);
  const [available, setAvailable] = useState<{ path: string; name: string }[]>([]);

  useEffect(() => {
    if (!choosing) return;
    void listWorkspace().then((graphs) =>
      setAvailable(
        graphs.map((g) => ({ path: g.path, name: g.name || (g.path.split('/').pop() ?? g.path) })),
      ),
    );
  }, [choosing]);

  if (open.length === 0) return null;

  return (
    <div className={s.tabs} data-testid="tabs">
      {open.map((tab) => (
        <span
          key={tab.path}
          className={`${s.tab} ${tab.path === active ? s.active : ''}`}
          data-testid={`tab-${tab.path.split('/').pop()}`}
        >
          <button type="button" className={s.name} onClick={() => void show(tab.path)}>
            {tab.name}
          </button>
          {/* Closing the last tab would leave the window with nothing to draw,
              so the last one has no close button rather than a broken one. */}
          {open.length > 1 && (
            <button
              type="button"
              className={s.close}
              title={`Close ${tab.name}`}
              onClick={() => close(tab.path)}
            >
              <Icon name="minus" size={10} strokeWidth={1.8} />
            </button>
          )}
        </span>
      ))}

      <span className={s.adder}>
        <button
          type="button"
          className={s.add}
          title="Open another graph"
          onClick={() => setChoosing((was) => !was)}
        >
          <Icon name="plus" size={11} strokeWidth={1.8} />
        </button>
        {choosing && (
          <div className={s.menu}>
            {available
              .filter((g) => !open.some((t) => t.path === g.path))
              .map((g) => (
                <button
                  key={g.path}
                  type="button"
                  className={s.choice}
                  onClick={() => {
                    setChoosing(false);
                    void show(g.path);
                  }}
                >
                  {g.name}
                </button>
              ))}
            {available.every((g) => open.some((t) => t.path === g.path)) && (
              <div className={s.empty}>every graph in this workspace is open</div>
            )}
          </div>
        )}
      </span>

      {dirtyPath !== active && active !== null && <span className={s.hidden} />}
    </div>
  );
}
