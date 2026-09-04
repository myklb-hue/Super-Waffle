import { useEffect, useRef, useState } from 'react';
import { Chip, Icon } from '@cyberloom/ui';
import { kind as lookupKind } from '@cyberloom/graph-core';
import { useDocument } from '../stores/document';
import { CONSOLE_LIMIT, elapsed, useRun } from '../stores/run';
import s from './Console.module.css';

type Tab = 'console' | 'trace' | 'variables';

/**
 * The drawer under the canvas (SPEC §8.6).
 *
 * Three tabs on the same run: **Console** is what happened in order, **Trace**
 * is the tool calls with what went in and what came back, and **Variables** is
 * what each block's ports are holding now. They are three readings of one
 * event stream rather than three sources, so nothing here can disagree with
 * the canvas.
 */
export function Console() {
  const [tab, setTab] = useState<Tab>('console');
  const [open, setOpen] = useState(false);
  const lines = useRun((r) => r.lines);
  const trace = useRun((r) => r.trace);
  const phase = useRun((r) => r.phase);
  const clear = useRun((r) => r.clearConsole);
  const warnings = lines.filter((l) => l.level === 'warn').length;

  // A run opens the drawer the first time it has something to say. After that
  // it is the user's: reopening it on every event would fight anyone who
  // closed it mid-run.
  const opened = useRef(false);
  useEffect(() => {
    if (phase === 'running' && !opened.current) {
      opened.current = true;
      setOpen(true);
    }
    if (phase === 'idle') opened.current = false;
  }, [phase]);

  return (
    <section className={`${s.drawer} ${open ? s.open : ''}`}>
      <header className={s.tabs}>
        {(['console', 'trace', 'variables'] as const).map((name) => (
          <button
            key={name}
            type="button"
            className={`${s.tab} ${tab === name && open ? s.tabActive : ''}`}
            onClick={() => {
              setTab(name);
              setOpen(true);
            }}
          >
            {name[0]!.toUpperCase() + name.slice(1)}
            {name === 'trace' && trace.length > 0 && (
              <span className={s.count}>{trace.length}</span>
            )}
          </button>
        ))}
        <span className={s.spacer} />
        {warnings > 0 && (
          <Chip label={`${warnings} warning${warnings === 1 ? '' : 's'}`} color="warn" dot />
        )}
        <button type="button" className={s.link} onClick={clear} disabled={lines.length === 0}>
          clear
        </button>
        <button
          type="button"
          className={s.chevron}
          onClick={() => setOpen((was) => !was)}
          aria-label={open ? 'Collapse the console' : 'Open the console'}
        >
          <Icon name="chev" size={12} />
        </button>
      </header>

      {open && (
        <div className={`${s.body} cl-scroll`}>
          {tab === 'console' && <Lines />}
          {tab === 'trace' && <Trace />}
          {tab === 'variables' && <Variables />}
        </div>
      )}
    </section>
  );
}

/** `time · source · message`, the source coloured by category (SPEC §8.6). */
function Lines() {
  const lines = useRun((r) => r.lines);
  const graph = useDocument((d) => d.graph);
  const foot = useRef<HTMLDivElement>(null);

  // Follow the tail, which is what a console is for.
  useEffect(() => {
    foot.current?.scrollIntoView({ block: 'end' });
  }, [lines.length]);

  if (lines.length === 0) {
    return <p className={s.empty}>Nothing yet. Press Run.</p>;
  }
  return (
    <div className={s.lines}>
      {lines.length >= CONSOLE_LIMIT && (
        <p className={s.trimmed}>
          showing the last {CONSOLE_LIMIT} lines; earlier ones were dropped
        </p>
      )}
      {lines.map((line, i) => {
        const block = graph.blocks.find((b) => b.id === line.source);
        const category = block ? (lookupKind(block.kind)?.category ?? 'custom') : null;
        return (
          <div key={i} className={s.line}>
            <span className={s.at}>{elapsed(line.at)}</span>
            <span
              className={s.source}
              style={category ? { color: `var(--cat-${category})` } : undefined}
            >
              {line.source ?? 'graph'}
            </span>
            <span className={line.level === 'error' ? s.error : line.level === 'warn' ? s.warn : ''}>
              {line.message}
            </span>
          </div>
        );
      })}
      <div ref={foot} />
    </div>
  );
}

/** Every tool call: who asked, what for, and what came back. */
function Trace() {
  const trace = useRun((r) => r.trace);
  if (trace.length === 0) {
    return <p className={s.empty}>No tool calls. A model calls a tool when it wants one.</p>;
  }
  return (
    <div className={s.lines}>
      {trace.map((row, i) => (
        <div key={i} className={s.call}>
          <div className={s.line}>
            <span className={s.at}>{elapsed(row.at)}</span>
            <span className={s.source}>{row.caller}</span>
            <span className={s.name}>{row.name}</span>
            <span className={s.args}>{JSON.stringify(row.arguments)}</span>
          </div>
          <div className={s.line}>
            <span className={s.at} />
            <span className={s.source} />
            <span className={row.ok === false ? s.error : s.result}>
              {row.result === null ? 'waiting…' : row.result}
            </span>
            {row.ms !== null && <span className={s.ms}>{row.ms} ms</span>}
          </div>
        </div>
      ))}
    </div>
  );
}

/** What every block's ports are holding, which is the run made inspectable
 *  rather than merely watchable. */
function Variables() {
  const blocks = useRun((r) => r.blocks);
  const results = useRun((r) => r.results);
  const entries = Object.entries(blocks).filter(([, b]) => b.output || b.figure || b.error);

  if (entries.length === 0 && results.length === 0) {
    return <p className={s.empty}>Nothing has produced a value yet.</p>;
  }
  return (
    <div className={s.lines}>
      {results.map((result) => (
        <div key={`out-${result.port}`} className={s.line}>
          <span className={s.at} />
          <span className={s.source}>result</span>
          <span className={s.name}>{result.port}</span>
          <span className={s.args}>{summary(result.value)}</span>
        </div>
      ))}
      {entries.map(([id, block]) => (
        <div key={id} className={s.line}>
          <span className={s.at} />
          <span className={s.source}>{id}</span>
          <span className={block.error ? s.error : s.args}>
            {block.error ?? block.figure ?? block.output.slice(0, 200)}
          </span>
        </div>
      ))}
    </div>
  );
}

function summary(value: { type: string; value?: unknown }): string {
  if (value.type === 'null') return '—';
  const text = typeof value.value === 'string' ? value.value : JSON.stringify(value.value);
  const flat = (text ?? '').replace(/\s+/g, ' ').trim();
  return flat.length > 160 ? `${flat.slice(0, 159)}…` : flat;
}
