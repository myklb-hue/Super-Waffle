import {
  CATEGORIES,
  KINDS,
  kindsIn,
  type Graph,
  type PortType,
} from '@cyberloom/graph-core';
import { Chip, Icon, StatusDot, TypeDots, type IconName } from '@cyberloom/ui';
import { ReactFlowProvider } from '@xyflow/react';
import { Canvas } from '../canvas/Canvas';
import s from './Shell.module.css';

export interface ShellProps {
  graph: Graph;
  path: string;
  /** What the engine said is wrong with this graph. Shown, never fatal. */
  problems: string[];
  engine: { state: 'starting' | 'ready' | 'unreachable'; detail: string };
}

/**
 * The whole window: library on the left, canvas in the middle, inspector on the
 * right, transport across the top and state along the bottom.
 *
 * This slice draws it and nothing more. The library does not drag, the
 * inspector shows the graph and not a selection, and no block can move.
 */
export function Shell({ graph, path, problems, engine }: ShellProps) {
  return (
    <div className={s.shell}>
      <TopBar graph={graph} path={path} />
      <div className={s.middle}>
        <Library />
        <ReactFlowProvider>
          <Canvas graph={graph} />
        </ReactFlowProvider>
        <Inspector graph={graph} problems={problems} />
      </div>
      <StatusBar graph={graph} engine={engine} problems={problems} />
    </div>
  );
}

function TopBar({ graph, path }: { graph: Graph; path: string }) {
  return (
    <header className={s.topbar}>
      <span className={s.mark}>
        <Icon name="mark" size={18} color="accent" strokeWidth={1.7} />
      </span>
      <span className={s.filename}>{path.split('/').pop()}</span>
      <span className={s.saved}>saved</span>

      <div className={s.transport}>
        <button type="button" className={s.run} disabled>
          <Icon name="play" size={11} strokeWidth={0} />
          Run
        </button>
        <Chip label={`local · ${graph.defaults.provider}`} color="ok" dot size="md" />
      </div>

      <div className={s.topRight}>
        <Chip label={graph.localOnly ? 'local only' : 'remote allowed'} color="text-mid" />
        <button type="button" className={s.iconButton} aria-label="More" disabled>
          <Icon name="dots" size={14} />
        </button>
      </div>
    </header>
  );
}

/**
 * The library, read-only in this slice: every category, every kind, and the
 * types each one speaks. Dragging one onto the canvas is slice 3.
 */
function Library() {
  return (
    <aside className={s.library}>
      <div className={s.search}>
        <Icon name="search" size={13} color="text-low" />
        <span className={s.searchText}>Search blocks</span>
        <kbd className={s.kbd}>⌘K</kbd>
      </div>
      <div className={`${s.shelves} cl-scroll`}>
        {CATEGORIES.map((category) => {
          const kinds = kindsIn(category);
          if (kinds.length === 0) return null;
          return (
            <section key={category} className={s.shelf}>
              <header className={s.shelfHead} style={{ ['--cat' as string]: `var(--cat-${category})` }}>
                <span className={s.shelfDot} />
                <span className={s.shelfName}>{category}</span>
                <span className={s.shelfCount}>{kinds.length}</span>
              </header>
              {kinds.map((kind) => (
                <div key={kind.id} className={s.row} title={kind.summary}>
                  <Icon
                    name={kind.icon as IconName}
                    size={13}
                    color={`cat-${category}`}
                    strokeWidth={1.7}
                  />
                  <span className={s.rowName}>{kind.title}</span>
                  <TypeDots kinds={typesOf(kind.ports)} />
                </div>
              ))}
            </section>
          );
        })}
      </div>
      <footer className={s.libraryFoot}>
        <Icon name="plus" size={12} color="text-low" />
        <span>New custom block</span>
      </footer>
    </aside>
  );
}

/**
 * The inspector, which in this slice always shows the graph, because there is
 * nothing to select yet. The rule it will follow is that the panel is the state
 * of the canvas (SPEC §7.1).
 */
function Inspector({ graph, problems }: { graph: Graph; problems: string[] }) {
  return (
    <aside className={`${s.inspector} cl-scroll`}>
      <header className={s.panelHead}>
        <span className={s.panelIcon}>
          <Icon name="mark" size={14} color="accent" strokeWidth={1.7} />
        </span>
        <span>
          <div className={s.panelTitle}>Graph</div>
          <div className={s.panelSub}>{graph.id}</div>
        </span>
      </header>

      {problems.length > 0 && (
        <section className={s.problems}>
          <div className={s.problemsTitle}>{problems.length} to look at</div>
          {problems.map((p) => (
            <div key={p} className={s.problem}>
              {p}
            </div>
          ))}
        </section>
      )}

      <Section title="Graph">
        <Field label="Name" value={graph.name} mono />
        {graph.description && <Field label="Description" value={graph.description} />}
      </Section>

      <Section title="Execution">
        <Field label="Run mode" value={graph.runMode} mono />
        <Field label="Runtime" value={graph.execution.runtime} mono />
        <Field label="Concurrency" value={`${graph.execution.concurrency} parallel`} mono />
        <Field label="Timeout" value={`${graph.execution.timeoutSec} s`} mono />
      </Section>

      <Section title="Defaults">
        <Field label="Provider" value={graph.defaults.provider} mono />
        <Field label="Model" value={graph.defaults.model} mono />
      </Section>

      <Section title="Overlap">
        <Field label="Policy" value={graph.overlap.policy} mono />
        <Field label="Max queue" value={String(graph.overlap.maxQueue)} mono />
      </Section>
    </aside>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className={s.section}>
      <div className={s.sectionLabel}>{title}</div>
      {children}
    </section>
  );
}

function Field({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className={s.field}>
      <div className={s.fieldLabel}>{label}</div>
      <div className={mono ? s.fieldValueMono : s.fieldValue}>{value}</div>
    </div>
  );
}

function StatusBar({
  graph,
  engine,
  problems,
}: {
  graph: Graph;
  engine: ShellProps['engine'];
  problems: string[];
}) {
  return (
    <footer className={s.statusbar}>
      <span className={s.status}>
        {graph.blocks.length} blocks · {graph.wires.length} wires
        {graph.frames.length > 0 &&
          ` · ${graph.frames.length} frame${graph.frames.length === 1 ? '' : 's'}`}
      </span>
      {problems.length > 0 && (
        <span className={s.statusWarn}>
          {problems.length} problem{problems.length === 1 ? '' : 's'}
        </span>
      )}
      <span className={s.statusRight}>
        <StatusDot state={engineDot(engine.state)} />
        engine: {engine.detail} · {KINDS.length} kinds
      </span>
    </footer>
  );
}

function engineDot(state: ShellProps['engine']['state']) {
  if (state === 'ready') return 'ok' as const;
  if (state === 'unreachable') return 'error' as const;
  return 'queued' as const;
}

/** The distinct types a kind speaks, in the order it declares them. */
function typesOf(ports: readonly { type: PortType }[]): PortType[] {
  const seen: PortType[] = [];
  for (const p of ports) if (!seen.includes(p.type)) seen.push(p.type);
  return seen.slice(0, 4);
}
