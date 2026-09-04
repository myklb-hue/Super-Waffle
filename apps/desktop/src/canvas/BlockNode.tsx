import { Handle, Position as FlowPosition, type NodeProps } from '@xyflow/react';
import {
  BLOCK_MIN_WIDTH,
  PORT_ROW,
  accepts,
  hasStage,
  kind as lookupKind,
  portY,
  thirdView,
  visiblePorts,
} from '@cyberloom/graph-core';
import type { Block, BlockState, Port, View } from '@cyberloom/graph-core';
import type { StatusState } from '@cyberloom/ui';
import { Chip, Grip, Icon, StatusDot, TypeDot, ViewToggle, type IconName } from '@cyberloom/ui';
import { Face } from '../avatar/Face';
import { Editor } from '../code/Editor';
import { useDocument } from '../stores/document';
import { useRun, type BlockRun } from '../stores/run';
import { ago, useSource } from '../stores/source';
import type { DragState } from './Canvas';
import s from './BlockNode.module.css';

export interface BlockNodeData extends Record<string, unknown> {
  block: Block;
  /** Which of this block's ports have a wire on them. */
  wired: Set<string>;
  /** The wire drag in flight, if there is one, so ports can dim. */
  drag: DragState | null;
}

/**
 * One block on the canvas.
 *
 * The geometry is not free: `portY` from graph-core decides where every port
 * row sits, and the wire router uses the same function, which is what makes a
 * wire land on a dot rather than near one. Nothing here computes a position of
 * its own.
 */
export function BlockNode({ data, selected }: NodeProps & { data: BlockNodeData }) {
  const { block, wired, drag } = data;
  // Subscribed per block rather than to the whole run: a chunk of a model's
  // output arrives many times a second, and every block on the canvas
  // re-rendering for each one is the difference between a live graph and a
  // slideshow.
  const live = useRun((r) => r.blocks[block.id]);
  // A custom block whose code will not parse goes red (SPEC §10.4). Its ports
  // and its wires stay exactly as they were, so the graph runs around it.
  const broken = useSource((s) => !!s.blocks[block.id]?.error);
  const armed = useRun((r) => r.armed[block.id]);
  const setView = useDocument((d) => d.setBlockView);
  const resize = useDocument((d) => d.resizeBlock);
  const kind = lookupKind(block.kind);
  const category = kind?.category ?? 'custom';
  const colour = `var(--cat-${category})`;
  const title = block.title ?? kind?.title ?? block.kind;

  // A custom block carries its ports on itself; everything else takes the
  // catalogue's (SPEC §10.2).
  const declared: Port[] =
    block.kind === 'custom'
      ? block.ports
      : (kind?.ports.map((p) => ({
          name: p.name,
          type: p.type,
          side: p.side,
          optional: p.optional,
        })) ?? []);

  const ins = visiblePorts(declared.filter((p) => p.side === 'in'), wired, selected);
  const outs = visiblePorts(declared.filter((p) => p.side === 'out'), wired, selected);
  const rows = Math.max(ins.length, outs.length);
  const width = block.size?.w ?? BLOCK_MIN_WIDTH;
  const third = thirdView(block);
  // The toggle and the grip appear on hover or selection, so a canvas at rest
  // is not covered in controls (SPEC §3.4). Hover is CSS; selection is this.
  const showControls = selected;
  const resizable = block.view === 'stage' || block.view === 'code' || hasStage(block.kind);

  return (
    <div
      className={[
        s.block,
        selected && s.selected,
        broken && s.broken,
        live?.state === 'running' && s.running,
        reporting(live) && s.reporting,
      ]
        .filter(Boolean)
        .join(' ')}
      style={{ width, ['--cat' as string]: colour }}
      data-testid={`block-${block.id}`}
      data-kind={block.kind}
    >
      <header
        className={s.header}
        // Double-clicking the header cycles the views (SPEC §2.4). The toggle
        // does the same thing with a click; this is the gesture for someone
        // who is already dragging the block around by its header.
        onDoubleClick={(e) => {
          e.stopPropagation();
          setView(block.id, nextView(block.view, third));
        }}
      >
        <Icon name={(kind?.icon ?? 'note') as IconName} size={12} strokeWidth={1.7} />
        <span className={s.title}>{title}</span>
        {block.kind === 'custom' && block.source && (
          <Chip label={shortLanguage(block.source.language)} color="cat-custom" />
        )}
        <span className={`${s.toggle} ${showControls ? s.toggleShown : ''}`}>
          <ViewToggle
            active={block.view}
            third={third}
            onChange={(view) => setView(block.id, view)}
          />
        </span>
        {live?.state === 'running' && block.kind === 'llm' && (
          <Chip label="streaming" color="ok" dot />
        )}
        {/* An armed source says what it is watching, which is the difference
            between a graph that is up and one that merely started (SPEC §8.2). */}
        {armed && <Chip label={armed} color="ok" dot />}
        <StatusDot state={block.disabled ? 'off' : dotFor(live?.state)} />
      </header>

      {/* The port zone comes before the body, so a label never overlays
          content (SPEC §3.1). Rows are absolutely placed from portY. */}
      <div className={s.ports} style={{ height: rows * PORT_ROW }}>
        {ins.map((port, i) => (
          <PortRow
            key={`in-${port.name}`}
            port={port}
            index={i}
            side="in"
            blockId={block.id}
            drag={drag}
          />
        ))}
        {outs.map((port, i) => (
          <PortRow
            key={`out-${port.name}`}
            port={port}
            index={i}
            side="out"
            blockId={block.id}
            drag={drag}
          />
        ))}
      </div>

      {block.view !== 'compact' && <Body block={block} />}
      {block.kind === 'custom' && block.view !== 'compact' && <InterfaceStrip block={block} />}

      {/* What the block is doing, while it is doing it (SPEC §3.2). */}
      {live && <Figures live={live} />}

      {resizable && (
        <span className={`${s.gripSlot} ${showControls ? s.toggleShown : ''}`}>
          <Grip
            onResize={(dx, dy) =>
              resize(
                block.id,
                (block.size?.w ?? BLOCK_MIN_WIDTH) + dx,
                block.size?.h ? block.size.h + dy : null,
              )
            }
          />
        </span>
      )}
    </div>
  );
}

/**
 * The next view in the cycle: compact → summary → the block's third view.
 *
 * A block with no third view cycles between two, which is why this cannot be
 * a fixed list of three (SPEC §3.4).
 */
function nextView(current: View, third: 'code' | 'stage' | null): View {
  const cycle: View[] = third ? ['compact', 'summary', third] : ['compact', 'summary'];
  const at = cycle.indexOf(current);
  return cycle[(at + 1) % cycle.length] ?? 'summary';
}

/** Whether this block has anything to say below itself. */
function reporting(live: BlockRun | undefined): boolean {
  return !!live && (live.state === 'running' || !!live.figure || !!live.error || !!live.output);
}

/** How the engine's block state draws as a dot.
 *
 *  The two vocabularies are nearly the same and deliberately not identical:
 *  the engine says `done` and `disabled`, the canvas draws `ok` and `off`,
 *  because a dot is about appearance and a state is about what happened. */
function dotFor(state: BlockState | undefined): StatusState {
  switch (state) {
    case 'running':
      return 'running';
    case 'done':
      return 'ok';
    case 'error':
      return 'error';
    case 'queued':
      return 'queued';
    case 'ready':
      return 'ready';
    case 'disabled':
      return 'off';
    default:
      return 'idle';
  }
}

/** The live figures under a running block: what it produced, or how it failed.
 *
 *  Only while there is something to say. A canvas that has never been run shows
 *  none of this, which is what keeps a graph at rest readable (SPEC §3.4). */
function Figures({ live }: { live: BlockRun }) {
  if (live.error) {
    return (
      <div className={`${s.figure} ${s.figureError}`} title={live.error}>
        {live.error}
      </div>
    );
  }
  if (!live.figure && !live.output) return null;
  return (
    <div className={s.figure}>
      {live.figure && <span className={s.figureText}>{live.figure}</span>}
      {live.output && !live.figure && (
        <span className={s.figureStream}>{tail(live.output, 48)}</span>
      )}
    </div>
  );
}

/** The last of a stream, so a block shows what is arriving rather than what
 *  arrived first and then stopped moving. */
function tail(text: string, width: number): string {
  const flat = text.replace(/\s+/g, ' ').trimEnd();
  return flat.length <= width ? flat : `…${flat.slice(-width)}`;
}

/** One port: a dot on the edge, a label inside it. */
function PortRow({
  port,
  index,
  side,
  blockId,
  drag,
}: {
  port: Port;
  index: number;
  side: 'in' | 'out';
  blockId: string;
  drag: DragState | null;
}) {
  // portY is measured from the block's top; the port zone starts under the
  // header, so the offset within the zone is what is left.
  const top = portY(index) - portY(0) + PORT_ROW / 2;
  const state = dragState(drag, blockId, port, side);
  return (
    <div
      className={[
        side === 'in' ? s.portIn : s.portOut,
        state === 'dim' && s.dimmed,
        state === 'target' && s.glowing,
      ]
        .filter(Boolean)
        .join(' ')}
      style={{ top }}
      data-port={`${blockId}.${port.name}`}
    >
      <Handle
        id={port.name}
        type={side === 'in' ? 'target' : 'source'}
        position={side === 'in' ? FlowPosition.Left : FlowPosition.Right}
        className={s.dot}
        style={{ ['--ty' as string]: `var(--type-${port.type})` }}
      />
      <span className={s.portLabel}>{port.name}</span>
    </div>
  );
}

/**
 * How a port should look while a wire is being dragged: lit if it can accept
 * the drag, dimmed if it cannot, and neither when nothing is being dragged.
 * The block the drag started from is left alone, so its own ports do not dim.
 */
function dragState(
  drag: DragState | null,
  blockId: string,
  port: Port,
  side: 'in' | 'out',
): 'target' | 'dim' | null {
  if (!drag) return null;
  if (drag.node === blockId) return null;
  // A drag from an output looks for inputs, and the other way round.
  if (side === drag.side) return 'dim';
  const [from, to] = drag.side === 'out' ? [drag.type, port.type] : [port.type, drag.type];
  return accepts(from, to) ? 'target' : 'dim';
}

/**
 * The inline preview: a field, a value, whatever the block wants to show. It is
 * a preview, not the settings; settings live in the inspector (SPEC §3.1).
 */
function Body({ block }: { block: Block }) {
  const kind = lookupKind(block.kind);
  const primary = kind?.settings[0]?.name;
  const value = primary ? block.settings[primary] : undefined;

  if (block.kind === 'custom') return <CustomBody block={block} />;
  if (block.kind === 'avatar') return <AvatarBody block={block} />;

  if (value === undefined || value === null) return null;

  return (
    <div className={s.body}>
      {primary && <div className={s.label}>{primary}</div>}
      <div className={s.value}>{preview(value)}</div>
    </div>
  );
}

/**
 * The Avatar's body: a thumbnail in Summary, the rig itself in Stage
 * (SPEC §11.6).
 *
 * Stage costs real canvas room, which is the trade the view toggle exists to
 * let a person make. Summary shows the face small with what it is doing beside
 * it, so a busy graph still says whether the assistant is smiling.
 */
function AvatarBody({ block }: { block: Block }) {
  const face = useRun((r) => r.faces[block.id]);
  const rig = face?.rig ?? (block.settings.rig as string) ?? 'line';
  const stage = block.view === 'stage';
  const size = stage ? (block.size?.w ?? BLOCK_MIN_WIDTH) - 24 : 44;

  return (
    <div className={stage ? s.stage : s.body}>
      <Face
        rig={rig}
        expression={face?.expression}
        intensity={face?.intensity}
        mouth={face?.mouth}
        gaze={face?.gaze}
        size={size}
      />
      {!stage && (
        <div className={s.stageSide}>
          <div className={s.label}>rig</div>
          <div className={s.value}>{rig}</div>
        </div>
      )}
    </div>
  );
}

/**
 * A custom block's body: the code, or a summary of it.
 *
 * Code view holds a real editor; summary holds the first lines and a count.
 * Past a screenful the code leaves the block entirely for the drawer
 * (SPEC §10.7), and this says so rather than showing a scrolling sliver of a
 * 184-line function in a 300px box.
 */
export const DRAWER_AFTER_LINES = 24;

function CustomBody({ block }: { block: Block }) {
  const setCode = useDocument((d) => d.setCode);
  const setSourceMode = useDocument((d) => d.setSourceMode);
  const source = useSource((s) => s.blocks[block.id]);
  const reload = useSource((s) => s.reload);
  const schedule = useSource((s) => s.schedule);

  const code = block.source?.code ?? '';
  const lines = code.split('\n');
  const language = block.source?.language ?? 'python';
  const inFile = block.source?.mode === 'file';

  if (block.view !== 'code') {
    return (
      <div className={s.body}>
        <div className={s.label}>{inFile ? (block.source?.path ?? 'file') : 'inline'}</div>
        <pre className={s.code}>{lines.slice(0, 3).join('\n')}</pre>
        <div className={s.more}>
          {lines.length} line{lines.length === 1 ? '' : 's'}
          {source?.at ? ` · reloaded ${ago(source.at)}` : ''}
        </div>
      </div>
    );
  }

  return (
    <div className={s.codeBody}>
      <div className={s.codeBar}>
        <span className={s.modes}>
          {(['inline', 'file'] as const).map((mode) => (
            <button
              key={mode}
              type="button"
              className={`${s.mode} ${block.source?.mode === mode ? s.modeOn : ''}`}
              onClick={() => setSourceMode(block.id, mode)}
            >
              {mode}
            </button>
          ))}
        </span>
        <Chip label={language} color="cat-custom" />
        {source?.error && <Chip label={`line ${source.error.line}`} color="err" dot />}
      </div>

      {lines.length > DRAWER_AFTER_LINES ? (
        // Past a screenful the code belongs in the drawer, and the block stays
        // summary-sized (SPEC §10.7).
        <div className={s.tooBig}>
          {lines.length} lines — too many for the block. Open the code drawer below.
        </div>
      ) : (
        <Editor
          value={code}
          language={language}
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
      )}
    </div>
  );
}

/**
 * The strip under a custom block: what its signature produced.
 *
 * `in frame ● image · out result ● data · set threshold ● float`, and when it
 * last reloaded. It is the answer to "what did my edit do to the block", which
 * is the question the whole of §10.1 exists to make answerable at a glance.
 */
function InterfaceStrip({ block }: { block: Block }) {
  const source = useSource((s) => s.blocks[block.id]);
  const derived = source?.interface;
  if (!derived) return null;

  return (
    <div className={s.interface}>
      <span className={s.interfaceLabel}>interface</span>
      {derived.ports.map((port) => (
        <span key={`${port.side}.${port.name}`} className={s.chipRow}>
          <span className={s.chipSide}>{port.side}</span>
          <span className={s.chipName}>{port.name}</span>
          <TypeDot kind={port.type} />
        </span>
      ))}
      {derived.settings.map((setting) => (
        <span key={setting.name} className={s.chipRow}>
          <span className={s.chipSide}>set</span>
          <span className={s.chipName}>{setting.name}</span>
        </span>
      ))}
      {source.note && <span className={s.note}>{source.note}</span>}
    </div>
  );
}

function preview(value: unknown): string {
  if (typeof value === 'string') return value.length > 64 ? `${value.slice(0, 63)}…` : value;
  if (Array.isArray(value)) return value.join(', ');
  return String(value);
}

function shortLanguage(language: string): string {
  return { python: 'py', typescript: 'ts', javascript: 'js', shell: 'sh' }[language] ?? language;
}
