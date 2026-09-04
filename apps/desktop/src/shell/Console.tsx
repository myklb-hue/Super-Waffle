import { useEffect, useRef, useState } from 'react';
import { Chip, Icon } from '@cyberloom/ui';
import { kind as lookupKind, type Block } from '@cyberloom/graph-core';
import { useDocument } from '../stores/document';
import { CONSOLE_LIMIT, elapsed, useRun } from '../stores/run';
import { ago, useSource } from '../stores/source';
import { Editor } from '../code/Editor';
import s from './Console.module.css';

type Tab = 'console' | 'trace' | 'variables' | 'code';

/**
 * The drawer under the canvas (SPEC §8.6).
 *
 * Three tabs on the same run: **Console** is what happened in order, **Trace**
 * is the tool calls with what went in and what came back, and **Variables** is
 * what each block's ports are holding now. They are three readings of one
 * event stream rather than three sources, so nothing here can disagree with
 * the canvas.
 *
 * A fourth tab appears when a custom block is selected: its code, full width
 * (SPEC §10.7). Past a screenful the code leaves the block and lives here, and
 * the block on the canvas stays summary-sized — which is the whole point,
 * because a 184-line function has no business being a card on a canvas.
 */
export function Console() {
  const [tab, setTab] = useState<Tab>('console');
  const [open, setOpen] = useState(false);
  const lines = useRun((r) => r.lines);
  const trace = useRun((r) => r.trace);
  const phase = useRun((r) => r.phase);
  const clear = useRun((r) => r.clearConsole);
  const warnings = lines.filter((l) => l.level === 'warn').length;

  // The code tab exists only while there is code to show in it.
  const graph = useDocument((d) => d.graph);
  const selection = useDocument((d) => d.selection);
  const selected =
    selection.kind === 'block' && selection.ids.length === 1
      ? graph.blocks.find((b) => b.id === selection.ids[0])
      : undefined;
  const editable = selected?.kind === 'custom' ? selected : undefined;
  const tabs: Tab[] = editable
    ? ['console', 'trace', 'variables', 'code']
    : ['console', 'trace', 'variables'];

  // A tab that has gone away cannot stay chosen.
  useEffect(() => {
    if (tab === 'code' && !editable) setTab('console');
  }, [tab, editable]);

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
    <section
      className={[s.drawer, open && s.open, open && tab === 'code' && s.tall]
        .filter(Boolean)
        .join(' ')}
    >
      <header className={s.tabs}>
        {tabs.map((name) => (
          <button
            key={name}
            type="button"
            className={`${s.tab} ${tab === name && open ? s.tabActive : ''}`}
            onClick={() => {
              setTab(name);
              setOpen(true);
            }}
          >
            {name === 'code' ? (editable?.title ?? 'Code') : name[0]!.toUpperCase() + name.slice(1)}
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
          {tab === 'code' && editable && <Code block={editable} />}
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

/**
 * A custom block's code, full width (SPEC §10.7).
 *
 * The same editor the block holds, in a place with room for it. It edits the
 * same document, so what is typed here and what is typed on the block are one
 * thing — there is no copy to keep in step.
 */
function Code({ block }: { block: Block }) {
  const setCode = useDocument((d) => d.setCode);
  const source = useSource((s) => s.blocks[block.id]);
  const reload = useSource((s) => s.reload);
  const schedule = useSource((s) => s.schedule);
  const code = block.source?.code ?? '';
  const inFile = block.source?.mode === 'file';

  return (
    <div className={s.code}>
      <div className={s.codeHead}>
        <span className={s.codePath}>
          {inFile ? (block.source?.path ?? 'a file') : `${block.title ?? block.id} · inline`}
        </span>
        <span className={s.codeState}>
          {source?.error
            ? `line ${source.error.line}: ${source.error.message}`
            : source?.at
              ? `reloaded ${ago(source.at)}${source.note ? ` · ${source.note}` : ' · interface unchanged'}`
              : 'not parsed yet'}
        </span>
      </div>
      <Editor
        value={code}
        language={block.source?.language ?? 'python'}
        errorLine={source?.error?.line ?? null}
        onChange={
          inFile
            ? undefined
            : (next) => {
                setCode(block.id, next);
                schedule(block.id);
              }
        }
        onSettle={() => void reload(block.id)}
        className={s.codeEditor}
      />
    </div>
  );
}

function summary(value: { type: string; value?: unknown }): string {
  if (value.type === 'null') return '—';
  const text = typeof value.value === 'string' ? value.value : JSON.stringify(value.value);
  const flat = (text ?? '').replace(/\s+/g, ' ').trim();
  return flat.length > 160 ? `${flat.slice(0, 159)}…` : flat;
}
