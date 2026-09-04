import { Handle, Position as FlowPosition, type NodeProps } from '@xyflow/react';
import {
  BLOCK_MIN_WIDTH,
  PORT_ROW,
  kind as lookupKind,
  portY,
  visiblePorts,
} from '@cyberloom/graph-core';
import type { Block, Port } from '@cyberloom/graph-core';
import { Chip, Icon, StatusDot, type IconName } from '@cyberloom/ui';
import s from './BlockNode.module.css';

export interface BlockNodeData extends Record<string, unknown> {
  block: Block;
  /** Which of this block's ports have a wire on them. */
  wired: Set<string>;
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
  const { block, wired } = data;
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

  return (
    <div
      className={`${s.block} ${selected ? s.selected : ''}`}
      style={{ width, ['--cat' as string]: colour }}
      data-testid={`block-${block.id}`}
      data-kind={block.kind}
    >
      <header className={s.header}>
        <Icon name={(kind?.icon ?? 'note') as IconName} size={12} strokeWidth={1.7} />
        <span className={s.title}>{title}</span>
        {block.kind === 'custom' && block.source && (
          <Chip label={shortLanguage(block.source.language)} color="cat-custom" />
        )}
        <StatusDot state={block.disabled ? 'off' : 'idle'} />
      </header>

      {/* The port zone comes before the body, so a label never overlays
          content (SPEC §3.1). Rows are absolutely placed from portY. */}
      <div className={s.ports} style={{ height: rows * PORT_ROW }}>
        {ins.map((port, i) => (
          <PortRow key={`in-${port.name}`} port={port} index={i} side="in" blockId={block.id} />
        ))}
        {outs.map((port, i) => (
          <PortRow key={`out-${port.name}`} port={port} index={i} side="out" blockId={block.id} />
        ))}
      </div>

      {block.view !== 'compact' && <Body block={block} />}
    </div>
  );
}

/** One port: a dot on the edge, a label inside it. */
function PortRow({
  port,
  index,
  side,
  blockId,
}: {
  port: Port;
  index: number;
  side: 'in' | 'out';
  blockId: string;
}) {
  // portY is measured from the block's top; the port zone starts under the
  // header, so the offset within the zone is what is left.
  const top = portY(index) - portY(0) + PORT_ROW / 2;
  return (
    <div
      className={side === 'in' ? s.portIn : s.portOut}
      style={{ top }}
      data-port={`${blockId}.${port.name}`}
    >
      <Handle
        id={port.name}
        type={side === 'in' ? 'target' : 'source'}
        position={side === 'in' ? FlowPosition.Left : FlowPosition.Right}
        className={s.dot}
        style={{ ['--ty' as string]: `var(--type-${port.type})` }}
        isConnectable={false}
      />
      <span className={s.portLabel}>{port.name}</span>
    </div>
  );
}

/**
 * The inline preview: a field, a value, whatever the block wants to show. It is
 * a preview, not the settings; settings live in the inspector (SPEC §3.1).
 */
function Body({ block }: { block: Block }) {
  const kind = lookupKind(block.kind);
  const primary = kind?.settings[0]?.name;
  const value = primary ? block.settings[primary] : undefined;

  if (block.kind === 'custom' && block.source?.code) {
    const lines = block.source.code.split('\n');
    return (
      <div className={s.body}>
        <div className={s.label}>{block.source.mode === 'file' ? 'file' : 'inline'}</div>
        <pre className={s.code}>{lines.slice(0, 3).join('\n')}</pre>
        {lines.length > 3 && <div className={s.more}>{lines.length} lines</div>}
      </div>
    );
  }

  if (value === undefined || value === null) return null;

  return (
    <div className={s.body}>
      {primary && <div className={s.label}>{primary}</div>}
      <div className={s.value}>{preview(value)}</div>
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
