import { Handle, Position as FlowPosition, type NodeProps } from '@xyflow/react';
import { PORT_ROW, kind as lookupKind, portY } from '@cyberloom/graph-core';
import type { Frame, PortDef } from '@cyberloom/graph-core';
import { Icon } from '@cyberloom/ui';
import s from './LoopFrame.module.css';

export interface LoopFrameData extends Record<string, unknown> {
  frame: Frame;
}

/**
 * A loop is a dashed frame on the canvas, not a card: the blocks inside it are
 * the body, and they repeat once per item (SPEC §3.4, §8.4).
 *
 * It is a node like any other because wires land on it — `items` in, `item`
 * out — so it has to be something xyflow can attach a handle to. It sits behind
 * the blocks, and it does not intercept the pointer, so clicking inside the
 * frame selects the block there rather than the frame.
 */
export function LoopFrame({ data, selected }: NodeProps & { data: LoopFrameData }) {
  const { frame } = data;
  const ports = lookupKind('loop')?.ports ?? [];
  const ins = ports.filter((p) => p.side === 'in');
  const outs = ports.filter((p) => p.side === 'out' && !p.optional);

  return (
    <div
      className={`${s.frame} ${selected ? s.selected : ''}`}
      style={{ width: frame.size.w, height: frame.size.h ?? 0 }}
      data-testid={`frame-${frame.id}`}
    >
      <header className={s.head}>
        <Icon name="loop" size={12} color="cat-control" strokeWidth={1.7} />
        <span className={s.title}>Loop</span>
        <span className={s.meta}>
          over {frame.over.node}.{frame.over.port} · as {frame.as} · {frame.parallel} at a time
        </span>
      </header>
      {ins.map((port, i) => (
        <FramePort key={port.name} port={port} index={i} side="in" />
      ))}
      {outs.map((port, i) => (
        <FramePort key={port.name} port={port} index={i} side="out" />
      ))}
    </div>
  );
}

function FramePort({
  port,
  index,
  side,
}: {
  port: PortDef;
  index: number;
  side: 'in' | 'out';
}) {
  return (
    <div className={side === 'in' ? s.portIn : s.portOut} style={{ top: portY(index) }}>
      <Handle
        id={port.name}
        type={side === 'in' ? 'target' : 'source'}
        position={side === 'in' ? FlowPosition.Left : FlowPosition.Right}
        className={s.dot}
        style={{ ['--ty' as string]: `var(--type-${port.type})` }}
        isConnectable={false}
      />
      <span className={s.portLabel} style={{ lineHeight: `${PORT_ROW}px` }}>
        {port.name}
      </span>
    </div>
  );
}
