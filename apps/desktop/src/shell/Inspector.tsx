import { useEffect } from 'react';
import {
  accepts,
  convertible,
  kind as lookupKind,
  type Block,
  type Generated,
  type Graph,
  type SettingDef,
} from '@cyberloom/graph-core';
import {
  Callout,
  Chip,
  Field,
  Icon,
  Label,
  Section,
  Segmented,
  Slider,
  SwitchRow,
  TextBox,
  TypeDot,
  type IconName,
} from '@cyberloom/ui';
import { portTypeOf, useDocument, type Selection } from '../stores/document';
import { useRun } from '../stores/run';
import { ago, useSource } from '../stores/source';
import { RunPanel } from './RunPanel';
import s from './Inspector.module.css';

/**
 * The inspector has no identity of its own: it is the state of the canvas
 * (SPEC §7.1). Nothing selected shows the graph, one block shows that block,
 * a wire shows that wire, several blocks show what they share. Same column,
 * different contents — which is why this is one component with four bodies
 * rather than four panels that happen to look alike.
 */
export function Inspector() {
  const graph = useDocument((d) => d.graph);
  const selection = useDocument((d) => d.selection);
  const problems = useDocument((d) => d.problems);
  const phase = useRun((r) => r.phase);
  // While a run is in flight the panel is the run's. The inspector has no
  // identity of its own and shows what matters now (SPEC §7.1); what matters
  // while a graph is running is the run, not what happens to be selected.
  const running = phase === 'running';

  return (
    <aside className={`${s.inspector} cl-scroll`}>
      {running ? <RunHead /> : <Head graph={graph} selection={selection} />}
      {!running && problems.length > 0 && <Problems problems={problems} />}
      {running ? <RunPanel /> : <Body graph={graph} selection={selection} />}
    </aside>
  );
}

function RunHead() {
  const run = useRun((r) => r.run);
  const usage = useRun((r) => r.usage);
  return (
    <header className={s.head} style={{ ['--c' as string]: 'var(--ok)' }}>
      <span className={s.icon}>
        <Icon name="play" size={12} color="ok" strokeWidth={0} />
      </span>
      <div className={s.titles}>
        <div className={s.title}>{run ?? 'Run'}</div>
        <div className={s.sub}>running · {usage?.local === false ? 'remote' : 'local'}</div>
      </div>
    </header>
  );
}

function Head({ graph, selection }: { graph: Graph; selection: Selection }) {
  let icon: IconName = 'mark';
  let colour = 'accent';
  let title = 'Graph';
  let sub = graph.id;

  if (selection.kind === 'block' && selection.ids.length === 1) {
    const block = graph.blocks.find((b) => b.id === selection.ids[0]);
    const kind = block && lookupKind(block.kind);
    icon = (kind?.icon ?? 'code') as IconName;
    colour = `cat-${kind?.category ?? 'custom'}`;
    title = block?.title ?? kind?.title ?? block?.kind ?? 'Block';
    sub = `${kind?.category ?? 'custom'} · ${block?.id ?? ''}`;
  } else if (selection.kind === 'block') {
    icon = 'chunk';
    title = `${selection.ids.length} blocks`;
    sub = 'what they share';
  } else if (selection.kind === 'wire') {
    const wire = graph.wires.find((w) => w.id === selection.id);
    icon = 'merge';
    title = 'Wire';
    sub = wire ? `${wire.from.node}.${wire.from.port} → ${wire.to.node}.${wire.to.port}` : '';
  } else if (selection.kind === 'frame') {
    icon = 'loop';
    colour = 'cat-control';
    title = 'Loop';
    sub = selection.id;
  }

  return (
    <header className={s.head} style={{ ['--c' as string]: `var(--${colour})` }}>
      <span className={s.icon}>
        <Icon name={icon} size={14} strokeWidth={1.7} />
      </span>
      <span className={s.titles}>
        <div className={s.title}>{title}</div>
        <div className={s.sub}>{sub}</div>
      </span>
    </header>
  );
}

function Problems({ problems }: { problems: string[] }) {
  return (
    <div className={s.problems}>
      <Callout
        title={`${problems.length} to look at`}
        body={problems.join('\n')}
        color="warn"
      />
    </div>
  );
}

function Body({ graph, selection }: { graph: Graph; selection: Selection }) {
  if (selection.kind === 'wire') return <WirePanel graph={graph} id={selection.id} />;
  if (selection.kind === 'frame') return <FramePanel graph={graph} id={selection.id} />;
  if (selection.kind === 'block') {
    if (selection.ids.length === 1) return <BlockPanel graph={graph} id={selection.ids[0]!} />;
    return <MultiPanel graph={graph} ids={selection.ids} />;
  }
  return <GraphPanel graph={graph} />;
}

/** Nothing selected: the panel falls back to graph-wide settings. */
function GraphPanel({ graph }: { graph: Graph }) {
  const setField = useDocument((d) => d.setGraphField);
  return (
    <>
      <Callout
        title="Nothing selected"
        body="The panel falls back to graph-wide settings. Select a block, a wire, or several blocks to change what appears here."
        color="text-low"
        dashed
      />
      <Section title="Graph">
        <Field value={graph.name} mono onChange={(v) => setField('name', v)} />
        <TextBox
          value={graph.description ?? ''}
          placeholder="What does this graph do?"
          onChange={(v) => setField('description', v || null)}
        />
      </Section>
      <Section title="Execution">
        <Label>Run mode</Label>
        <Segmented
          options={['once', 'live', 'schedule']}
          value={graph.runMode}
          label="Run mode"
          onChange={(v) => setField('runMode', v as Graph['runMode'])}
        />
        <Field value={graph.execution.runtime} icon="terminal" mono select onOpen={() => {}} />
        <Field
          value={String(graph.execution.concurrency)}
          suffix="parallel"
          mono
          onChange={(v) =>
            setField('execution', { ...graph.execution, concurrency: Number(v) || 1 })
          }
        />
        <Field
          value={String(graph.execution.timeoutSec)}
          suffix="s"
          mono
          onChange={(v) =>
            setField('execution', { ...graph.execution, timeoutSec: Number(v) || 0 })
          }
        />
      </Section>
      <Section title="Models" tint={graph.localOnly ? undefined : 'err'}>
        <Field value={graph.defaults.provider} mono select onOpen={() => {}} />
        <Field
          value={graph.defaults.model}
          mono
          onChange={(v) => setField('defaults', { ...graph.defaults, model: v })}
        />
        <SwitchRow
          label="Local only"
          hint="On: the graph may only use models on this machine. Turning it off allows remote providers; the first send of a run warns."
          on={graph.localOnly}
          color={graph.localOnly ? 'ok' : 'err'}
          onChange={(v) => setField('localOnly', v)}
        />
      </Section>
      <Section title="Overlap">
        <Label>When an event arrives mid-run</Label>
        <Segmented
          options={['queue', 'dropNewest', 'dropOldest', 'coalesce']}
          value={graph.overlap.policy}
          label="Overlap policy"
          onChange={(v) => setField('overlap', { ...graph.overlap, policy: v as never })}
        />
      </Section>
      <Section title="Env and secrets">
        {Object.keys(graph.env).length === 0 ? (
          <Callout
            title="No secrets bound"
            body="Add one to expose it to Terminal and HTTP blocks. Only the name is written to the file; the value lives in the OS keyring."
            color="text-low"
            dashed
          />
        ) : (
          Object.entries(graph.env).map(([name, ref]) => (
            <Field key={name} value={`${name} = ${ref}`} mono muted />
          ))
        )}
      </Section>
    </>
  );
}

/** One block: its own settings, generated from the kind's definitions. */
function BlockPanel({ graph, id }: { graph: Graph; id: string }) {
  const rename = useDocument((d) => d.renameBlock);
  const setSetting = useDocument((d) => d.setSetting);
  const toggleDisabled = useDocument((d) => d.toggleDisabled);
  const block = graph.blocks.find((b) => b.id === id);
  if (!block) return null;
  const kind = lookupKind(block.kind);

  return (
    <>
      <Section title="Block">
        <Label>Name</Label>
        <Field
          value={block.title ?? ''}
          placeholder={kind?.title ?? block.kind}
          onChange={(v) => rename(block.id, v)}
        />
        {kind && <div className={s.summary}>{kind.summary}</div>}
      </Section>

      {block.kind === 'custom' && <CustomSections block={block} />}

      {kind && kind.settings.length > 0 && (
        <Section title="Settings">
          {kind.settings.map((def) => (
            <SettingControl
              key={def.name}
              def={def}
              value={block.settings[def.name]}
              onChange={(v) => setSetting(block.id, def.name, v)}
            />
          ))}
        </Section>
      )}

      <Section title="Ports">
        <Ports graph={graph} block={block} />
      </Section>

      <Section title="On the canvas">
        <SwitchRow
          label="Enabled"
          hint="A disabled block is skipped; its wires are kept."
          on={!block.disabled}
          onChange={() => toggleDisabled([block.id])}
        />
      </Section>
    </>
  );
}

/**
 * The sections only a custom block has (SPEC §10).
 *
 * Source says where the code lives; Interface says what the signature made,
 * live; Settings are generated from the defaults the code wrote. The order is
 * the order of the questions: where is it, what did it produce, what can I
 * change.
 */
function CustomSections({ block }: { block: Block }) {
  const setSourceMode = useDocument((d) => d.setSourceMode);
  const setSetting = useDocument((d) => d.setSetting);
  const source = useSource((s) => s.blocks[block.id]);
  const reload = useSource((s) => s.reload);
  const derived = source?.interface;

  // Read once when the block is first shown, so the panel is not empty until
  // somebody types.
  useEffect(() => {
    if (!source) void reload(block.id);
  }, [block.id, source, reload]);

  const inFile = block.source?.mode === 'file';

  return (
    <>
      <Section title="Source">
        <Segmented
          options={['inline', 'file']}
          value={block.source?.mode ?? 'inline'}
          label="Source mode"
          onChange={(mode) => setSourceMode(block.id, mode as 'inline' | 'file')}
        />
        {inFile && (
          <>
            <Label>File</Label>
            <Field
              value={block.source?.path ?? ''}
              icon="folder"
              mono
              placeholder="~/blocks/door_check.py"
              onChange={(path) => setSourceMode(block.id, 'file', path)}
            />
          </>
        )}
        <div className={s.summary}>
          {inFile
            ? 'Edit in your own editor. The block reloads on every save and keeps its wires.'
            : 'The code lives in the graph file. Switch to File to edit it elsewhere.'}
        </div>
      </Section>

      <Section
        title="Interface"
        right={
          source?.error ? (
            <Chip label={`line ${source.error.line}`} color="err" dot />
          ) : (
            <Chip label="live" color="ok" dot />
          )
        }
      >
        {source?.error && (
          <Callout
            title={`Line ${source.error.line}`}
            body={source.error.message}
            color="err"
          />
        )}
        {derived?.ports.map((port) => (
          <div key={`${port.side}.${port.name}`} className={s.derived}>
            <span className={s.derivedSide}>{port.side}</span>
            <span className={s.derivedName}>{port.name}</span>
            <TypeDot kind={port.type} />
            <span className={s.derivedType}>{port.type}</span>
          </div>
        ))}
        {derived?.settings.map((setting) => (
          <div key={setting.name} className={s.derived}>
            <span className={s.derivedSide}>set</span>
            <span className={s.derivedName}>{setting.name}</span>
            <span className={s.derivedType}>= {setting.default}</span>
          </div>
        ))}
        <div className={s.summary}>
          {derived
            ? `parsed from the signature · ${ago(source?.at ?? null)}${
                source?.error ? ' · showing the last good read' : ' · no errors'
              }`
            : 'nothing parsed yet'}
        </div>
      </Section>

      {derived && derived.settings.length > 0 && (
        <Section title="Settings">
          {derived.settings.map((setting) => (
            <GeneratedControl
              key={setting.name}
              setting={setting}
              value={block.settings[setting.name]}
              onChange={(v) => setSetting(block.id, setting.name, v)}
            />
          ))}
          <div className={s.summary}>
            Generated from the default argument. Changing it here does not
            change the code; changing the code changes what this falls back to.
          </div>
        </Section>
      )}

      <Section title="Library">
        <Label>Category</Label>
        <Field value={derived?.category ?? 'custom'} mono />
        <Label>Icon</Label>
        <Field value={derived?.icon ?? '—'} mono />
        <div className={s.summary}>
          Both come from the `@block` decorator, so the shelf a block lands on
          is in its code like everything else about it.
        </div>
      </Section>
    </>
  );
}

/** One control for a setting the code asked for (SPEC §10.1). */
function GeneratedControl({
  setting,
  value,
  onChange,
}: {
  setting: Generated;
  value: unknown;
  onChange: (value: unknown) => void;
}) {
  // The default written in the code is the fallback, so an untouched setting
  // shows what the function would actually use rather than an empty box.
  const fallback = setting.default.replace(/^['"]|['"]$/g, '');

  if (setting.kind === 'bool') {
    return (
      <SwitchRow
        label={setting.label}
        on={value === undefined ? /true/i.test(fallback) : value === true}
        onChange={onChange}
      />
    );
  }
  if (setting.kind === 'range') {
    return (
      <Slider
        label={setting.label}
        value={typeof value === 'number' ? value : (Number(fallback) || 0)}
        min={setting.min ?? 0}
        max={setting.max ?? 1}
        step={0.01}
        onChange={onChange}
      />
    );
  }
  if (setting.kind === 'select' && setting.options.length > 0) {
    return (
      <>
        <Label>{setting.label}</Label>
        <Segmented
          options={[...setting.options]}
          value={typeof value === 'string' ? value : fallback}
          label={setting.label}
          onChange={onChange}
        />
      </>
    );
  }
  return (
    <>
      <Label>{setting.label}</Label>
      <Field
        value={value === undefined || value === null ? '' : String(value)}
        placeholder={fallback}
        mono={setting.kind === 'number' || setting.kind === 'path'}
        onChange={(v) => onChange(setting.kind === 'number' ? (Number(v) || 0) : v)}
      />
    </>
  );
}

/**
 * One control per setting, chosen by the kind's declaration rather than by a
 * hand-written panel per block.
 *
 * A setting the user has not touched shows the kind's declared default, and a
 * setting with no declared default shows nothing at all. Until the catalogue
 * carried defaults this panel filled the gap itself — a slider sat at its
 * minimum and a choice showed its first option — which read as a value someone
 * had chosen when nobody had. A control that is empty because nothing has been
 * decided should look empty.
 */
function SettingControl({
  def,
  value,
  onChange,
}: {
  def: SettingDef;
  value: unknown;
  onChange: (value: unknown) => void;
}) {
  if (def.kind === 'bool') {
    // A switch has no unset position, so the catalogue always declares one
    // (`every_switch_says_which_way_it_starts`).
    const on = value === undefined || value === null ? def.default === 'true' : value === true;
    return (
      <SwitchRow
        label={def.label}
        hint={def.hint ?? undefined}
        on={on}
        color={def.hint ? 'err' : 'accent'}
        onChange={onChange}
      />
    );
  }

  if (def.kind === 'range') {
    const fallback = def.default === null ? null : Number(def.default);
    const chosen = typeof value === 'number' ? value : fallback;
    return (
      <Slider
        label={def.label}
        // A slider with nothing behind it sits at its floor and says so, rather
        // than reading as a deliberate zero.
        value={chosen ?? (def.min ?? 0)}
        display={chosen === null ? 'unset' : undefined}
        min={def.min ?? 0}
        max={def.max ?? 1}
        step={(def.max ?? 1) - (def.min ?? 0) > 4 ? 1 : 0.01}
        onChange={onChange}
      />
    );
  }

  if (def.kind === 'select') {
    return (
      <>
        <Label>{def.label}</Label>
        <Segmented
          options={[...def.options]}
          value={typeof value === 'string' ? value : (def.default ?? def.options[0] ?? '')}
          label={def.label}
          onChange={onChange}
        />
      </>
    );
  }

  if (def.kind === 'multiline') {
    return (
      <>
        <Label>{def.label}</Label>
        <TextBox
          value={typeof value === 'string' ? value : ''}
          placeholder={def.default ?? undefined}
          mono
          onChange={onChange}
        />
      </>
    );
  }

  return (
    <>
      <Label>{def.label}</Label>
      <Field
        value={value === undefined || value === null ? '' : String(value)}
        // The declared default as a placeholder: what the block will use, shown
        // in the shape of something nobody typed.
        placeholder={def.default ?? undefined}
        mono={def.kind === 'path' || def.kind === 'number'}
        onChange={(v) => onChange(def.kind === 'number' ? Number(v) || 0 : v)}
      />
    </>
  );
}

function Ports({ graph, block }: { graph: Graph; block: Block }) {
  const kind = lookupKind(block.kind);
  const ports =
    block.kind === 'custom'
      ? block.ports
      : (kind?.ports.map((p) => ({
          name: p.name,
          type: p.type,
          side: p.side,
          optional: p.optional,
        })) ?? []);

  if (ports.length === 0) {
    return <div className={s.summary}>This block declares no ports yet.</div>;
  }

  return (
    <div className={s.ports}>
      {ports.map((port) => {
        const wires = graph.wires.filter(
          (w) =>
            (port.side === 'in' && w.to.node === block.id && w.to.port === port.name) ||
            (port.side === 'out' && w.from.node === block.id && w.from.port === port.name),
        );
        return (
          <div key={`${port.side}-${port.name}`} className={s.portRow}>
            <TypeDot kind={port.type} dim={wires.length === 0} />
            <span className={s.portName}>{port.name}</span>
            <span className={s.portMeta}>
              {port.side} · {port.type}
              {wires.length > 0 && ` · ${wires.length} wired`}
              {wires.length === 0 && port.optional && ' · optional'}
            </span>
          </div>
        );
      })}
    </div>
  );
}

/** A wire: its two ends, and whether they agree. */
function WirePanel({ graph, id }: { graph: Graph; id: string }) {
  const wire = graph.wires.find((w) => w.id === id);
  if (!wire) return null;
  const from = portTypeOf(graph, wire.from.node, wire.from.port, 'out');
  const to = portTypeOf(graph, wire.to.node, wire.to.port, 'in');
  const fits = !!from && !!to && accepts(from, to);
  const handle = from === 'tools' || from === 'memory';

  return (
    <>
      <Section title="Endpoints">
        <div className={s.endpoint}>
          {from && <TypeDot kind={from} />}
          <span className={s.portName}>
            {wire.from.node}.{wire.from.port}
          </span>
          <span className={s.portMeta}>out</span>
        </div>
        <div className={s.endpoint}>
          {to && <TypeDot kind={to} />}
          <span className={s.portName}>
            {wire.to.node}.{wire.to.port}
          </span>
          <span className={s.portMeta}>in</span>
        </div>
      </Section>

      <Section title="Type">
        <div className={s.chips}>
          {from && <Chip label={from} color={`type-${from}`} dot />}
          {handle && <Chip label="handle · two-way" color="text-mid" />}
        </div>
        {fits && from === to && (
          <div className={s.summary}>Exact match — no conversion needed.</div>
        )}
        {fits && from !== to && (
          <div className={s.summary}>
            {from} is accepted by {to}.
          </div>
        )}
        {!fits && from && to && (
          <Callout
            title={`${from} does not fit ${to}`}
            body={
              convertible(from, to)
                ? 'Insert a Convert block on the wire. A conversion is always something you can see.'
                : 'These two types have nothing in common. One of the endpoints has to change.'
            }
            color="err"
          />
        )}
      </Section>

      {handle && (
        <Section title="How it carries">
          <div className={s.summary}>
            A handle is two-way: the holder calls and the reply comes back on the same call.
            Anything the far end reports on its own initiative leaves on a port of its own.
          </div>
        </Section>
      )}
    </>
  );
}

/** Several blocks: what they share, and what can be changed for all of them. */
function MultiPanel({ graph, ids }: { graph: Graph; ids: string[] }) {
  const toggleDisabled = useDocument((d) => d.toggleDisabled);
  const blocks = graph.blocks.filter((b) => ids.includes(b.id));
  const kinds = [...new Set(blocks.map((b) => b.kind))];
  const categories = [...new Set(blocks.map((b) => lookupKind(b.kind)?.category ?? 'custom'))];
  const wiresBetween = graph.wires.filter(
    (w) => ids.includes(w.from.node) && ids.includes(w.to.node),
  );

  return (
    <>
      <Section title="Selection">
        <div className={s.chips}>
          {categories.map((c) => (
            <Chip key={c} label={c} color={`cat-${c}`} dot />
          ))}
        </div>
        <div className={s.summary}>
          {blocks.length} blocks, {kinds.length} {kinds.length === 1 ? 'kind' : 'kinds'},{' '}
          {wiresBetween.length} {wiresBetween.length === 1 ? 'wire' : 'wires'} between them.
        </div>
      </Section>

      <Section title="All of them">
        <SwitchRow
          label="Enabled"
          hint="Turns every block in the selection on or off together."
          on={blocks.some((b) => !b.disabled)}
          onChange={() => toggleDisabled(ids)}
        />
      </Section>

      <Section title="Blocks">
        <div className={s.ports}>
          {blocks.map((b) => {
            const kind = lookupKind(b.kind);
            return (
              <div key={b.id} className={s.portRow}>
                <Icon
                  name={(kind?.icon ?? 'code') as IconName}
                  size={12}
                  color={`cat-${kind?.category ?? 'custom'}`}
                />
                <span className={s.portName}>{b.title ?? kind?.title ?? b.kind}</span>
                <span className={s.portMeta}>{b.id}</span>
              </div>
            );
          })}
        </div>
      </Section>
    </>
  );
}

/** A loop frame: what it iterates and how fast. */
function FramePanel({ graph, id }: { graph: Graph; id: string }) {
  const frame = graph.frames.find((f) => f.id === id);
  if (!frame) return null;
  const inside = graph.blocks.filter((b) => b.frame === id);
  return (
    <>
      <Section title="Loop">
        <Label>Over</Label>
        <Field value={`${frame.over.node}.${frame.over.port}`} mono muted />
        <Label>Each item is called</Label>
        <Field value={frame.as} mono muted />
      </Section>
      <Section title="Pace">
        <Field value={String(frame.parallel)} suffix="at a time" mono muted />
        <Field value={String(frame.max)} suffix="max items" mono muted />
      </Section>
      <Section title="Inside">
        <div className={s.summary}>
          {inside.length} {inside.length === 1 ? 'block' : 'blocks'} repeat once per item.
        </div>
      </Section>
    </>
  );
}
