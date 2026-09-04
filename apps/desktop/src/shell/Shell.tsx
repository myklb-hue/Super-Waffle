import { useEffect, useMemo, useState } from 'react';
import { ReactFlowProvider } from '@xyflow/react';
import { CATEGORIES, KINDS, kindsIn, type PortType } from '@cyberloom/graph-core';
import { Chip, Icon, StatusDot, TypeDots, type IconName } from '@cyberloom/ui';
import { Canvas } from '../canvas/Canvas';
import { Console } from './Console';
import { Inspector } from './Inspector';
import { Settings } from './Settings';
import { Tabs } from './Tabs';
import { RunFigures, Transport, WarningPrompt } from './Transport';
import { useDocument } from '../stores/document';
import { useAutosave } from '../stores/autosave';
import { listenToRuns } from '../stores/run';
import { watchDocument } from '../stores/source';
import { useKeyboard } from './keyboard';
import s from './Shell.module.css';

export interface ShellProps {
  engine: { state: 'starting' | 'ready' | 'unreachable'; detail: string };
}

/**
 * The whole window: library on the left, canvas in the middle, inspector on the
 * right, transport across the top and state along the bottom.
 */
export function Shell({ engine }: ShellProps) {
  const save = useAutosave();
  const [settings, setSettings] = useState(false);
  useKeyboard();

  // Two subscriptions for the window: one to the engine's events, one to the
  // document. Every component reads the stores; nothing else listens.
  useEffect(() => listenToRuns(), []);
  useEffect(() => watchDocument(), []);

  return (
    <div className={s.shell}>
      <TopBar saveState={save.state} onSettings={() => setSettings(true)} />
      <Tabs />
      <div className={s.middle}>
        <Library />
        {/* The canvas and the console share a column: the drawer belongs to
            the graph being watched, not to the window (SPEC §8.6). */}
        <div className={s.centre}>
          <ReactFlowProvider>
            <Canvas />
          </ReactFlowProvider>
          <Console />
        </div>
        <Inspector />
      </div>
      <StatusBar engine={engine} saveError={save.error} />
      <WarningPrompt />
      {settings && <Settings onClose={() => setSettings(false)} />}
    </div>
  );
}

function TopBar({
  saveState,
  onSettings,
}: {
  saveState: 'saved' | 'saving' | 'dirty' | 'failed';
  onSettings: () => void;
}) {
  const graph = useDocument((d) => d.graph);
  const path = useDocument((d) => d.path);
  const label = {
    saved: 'saved',
    saving: 'saving…',
    dirty: 'edited',
    failed: 'not saved',
  }[saveState];

  return (
    <header className={s.topbar}>
      <span className={s.mark}>
        <Icon name="mark" size={18} color="accent" strokeWidth={1.7} />
      </span>
      <span className={s.filename}>{path.split('/').pop()}</span>
      <span className={saveState === 'failed' ? s.savedError : s.saved}>{label}</span>

      <div className={s.transport}>
        <Transport />
        <Chip label={`local · ${graph.defaults.provider}`} color="ok" dot size="md" />
      </div>

      <div className={s.topRight}>
        <Chip
          label={graph.localOnly ? 'local only' : 'remote allowed'}
          color={graph.localOnly ? 'text-mid' : 'warn'}
        />
        <button
          type="button"
          className={s.iconButton}
          aria-label="Workspace settings"
          title="Workspace settings"
          data-testid="open-settings"
          onClick={onSettings}
        >
          <Icon name="dots" size={14} />
        </button>
      </div>
    </header>
  );
}

/**
 * The library. A row is dragged onto the canvas to add that kind; the drop
 * itself is the canvas's business, so all this carries is the kind's id.
 */
function Library() {
  const [query, setQuery] = useState('');
  const matches = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return null;
    return KINDS.filter(
      (k) =>
        k.title.toLowerCase().includes(q) ||
        k.id.includes(q) ||
        k.summary.toLowerCase().includes(q),
    );
  }, [query]);

  return (
    <aside className={s.library}>
      <label className={s.search}>
        <Icon name="search" size={13} color="text-low" />
        <input
          className={s.searchInput}
          value={query}
          placeholder="Search blocks"
          onChange={(e) => setQuery(e.target.value)}
          data-search
        />
        {query ? (
          <button type="button" className={s.clear} onClick={() => setQuery('')} aria-label="Clear">
            <Icon name="minus" size={10} strokeWidth={2} />
          </button>
        ) : (
          <kbd className={s.kbd}>⌘K</kbd>
        )}
      </label>

      <div className={`${s.shelves} cl-scroll`}>
        {matches ? (
          <section className={s.shelf}>
            <header className={s.shelfHead}>
              <span className={s.shelfName}>{matches.length} found</span>
            </header>
            {matches.map((kind) => (
              <LibraryRow key={kind.id} kind={kind} />
            ))}
          </section>
        ) : (
          CATEGORIES.map((category) => {
            const kinds = kindsIn(category);
            if (kinds.length === 0) return null;
            return (
              <section key={category} className={s.shelf}>
                <header
                  className={s.shelfHead}
                  style={{ ['--cat' as string]: `var(--cat-${category})` }}
                >
                  <span className={s.shelfDot} />
                  <span className={s.shelfName}>{category}</span>
                  <span className={s.shelfCount}>{kinds.length}</span>
                </header>
                {kinds.map((kind) => (
                  <LibraryRow key={kind.id} kind={kind} />
                ))}
              </section>
            );
          })
        )}
      </div>

      <footer className={s.libraryFoot}>
        <Icon name="plus" size={12} color="text-low" />
        <span>New custom block</span>
      </footer>
    </aside>
  );
}

function LibraryRow({ kind }: { kind: (typeof KINDS)[number] }) {
  return (
    <div
      className={s.row}
      title={kind.summary}
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData('application/cyberloom-kind', kind.id);
        e.dataTransfer.effectAllowed = 'copy';
      }}
    >
      <Icon
        name={kind.icon as IconName}
        size={13}
        color={`cat-${kind.category}`}
        strokeWidth={1.7}
      />
      <span className={s.rowName}>{kind.title}</span>
      <TypeDots kinds={typesOf(kind.ports)} />
    </div>
  );
}

function StatusBar({
  engine,
  saveError,
}: {
  engine: ShellProps['engine'];
  saveError: string | null;
}) {
  const graph = useDocument((d) => d.graph);
  const problems = useDocument((d) => d.problems);
  const selection = useDocument((d) => d.selection);

  return (
    <footer className={s.statusbar}>
      <span>
        {graph.blocks.length} blocks · {graph.wires.length} wires
        {graph.frames.length > 0 &&
          ` · ${graph.frames.length} frame${graph.frames.length === 1 ? '' : 's'}`}
      </span>
      {selection.kind === 'block' && selection.ids.length > 1 && (
        <span>{selection.ids.length} selected</span>
      )}
      {problems.length > 0 && (
        <span className={s.statusWarn}>
          {problems.length} problem{problems.length === 1 ? '' : 's'}
        </span>
      )}
      {saveError && <span className={s.statusError}>{saveError}</span>}
      <span className={s.statusRight}>
        <RunFigures />
        <StatusDot state={engine.state === 'ready' ? 'ok' : engine.state === 'unreachable' ? 'error' : 'queued'} />
        engine: {engine.detail} · {KINDS.length} kinds
      </span>
    </footer>
  );
}

/** The distinct types a kind speaks, in the order it declares them. */
function typesOf(ports: readonly { type: PortType }[]): PortType[] {
  const seen: PortType[] = [];
  for (const p of ports) if (!seen.includes(p.type)) seen.push(p.type);
  return seen.slice(0, 4);
}

/** Focus the search box on Cmd/Ctrl+K, which is what the hint promises. */
export function useSearchShortcut() {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        document.querySelector<HTMLInputElement>('[data-search]')?.focus();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
}
