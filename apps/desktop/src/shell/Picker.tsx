import { useState } from 'react';
import { Button, Callout, Field, Icon, Label } from '@cyberloom/ui';
import s from './Picker.module.css';

/**
 * The workspace picker (`/`).
 *
 * A workspace is a folder (SPEC §15.6) and the engine serves exactly one, so
 * choosing another is starting a different engine rather than switching a mode.
 * The picker's whole job is to name the folder and hand it to the host.
 *
 * Recent workspaces are the shell's, not the engine's: they are a fact about
 * this person's habits rather than about any workspace, so they live in the
 * app's own storage and follow them between folders.
 */
const RECENT = 'cyberloom.recent';

export function recentWorkspaces(): string[] {
  try {
    const stored = JSON.parse(localStorage.getItem(RECENT) ?? '[]') as unknown;
    return Array.isArray(stored) ? stored.filter((p): p is string => typeof p === 'string') : [];
  } catch {
    // A browser with storage turned off is a browser with no history, not a
    // browser that cannot open a workspace.
    return [];
  }
}

export function remember(path: string): void {
  try {
    const kept = [path, ...recentWorkspaces().filter((p) => p !== path)].slice(0, 8);
    localStorage.setItem(RECENT, JSON.stringify(kept));
  } catch {
    /* nothing to remember with, and nothing that depends on it */
  }
}

export function Picker({ onOpen }: { onOpen: (path: string) => void }) {
  const [typed, setTyped] = useState('');
  const recent = recentWorkspaces();

  return (
    <div className={s.picker} data-testid="picker">
      <div className={s.panel}>
        <div className={s.mark}>
          <Icon name="mark" size={18} color="accent" strokeWidth={1.6} />
        </div>
        <div className={s.title}>Open a workspace</div>
        <div className={s.subtitle}>
          A workspace is a folder. Its graphs are the tabs; its library, rigs and
          settings are its own.
        </div>

        {recent.length > 0 && (
          <div className={s.recent}>
            <Label>Recent</Label>
            {recent.map((path) => (
              <button
                key={path}
                type="button"
                className={s.row}
                onClick={() => onOpen(path)}
                data-testid={`recent-${path}`}
              >
                <Icon name="folder" size={13} strokeWidth={1.7} />
                <span className={s.rowName}>{path.split('/').filter(Boolean).pop() ?? path}</span>
                <span className={s.rowPath}>{path}</span>
              </button>
            ))}
          </div>
        )}

        <Label>Folder</Label>
        <Field
          value={typed}
          placeholder="/home/you/graphs"
          mono
          onChange={setTyped}
        />
        <div className={s.actions}>
          <Button
            label="Open"
            variant="primary"
            disabled={typed.trim().length === 0}
            onClick={() => onOpen(typed.trim())}
          />
        </div>

        <Callout
          title="A folder is enough"
          body="No project file, no import step. Put .loom files in a folder and it is a workspace; a workspace.yaml appears the first time you change a setting."
          color="text-low"
          dashed
        />
      </div>
    </div>
  );
}
