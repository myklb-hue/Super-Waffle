import { BaseEdge, type EdgeProps } from '@xyflow/react';
import { controlOffset, wirePoint } from '@cyberloom/graph-core';
import type { PortType } from '@cyberloom/graph-core';
import s from './Wire.module.css';

export interface WireData extends Record<string, unknown> {
  type: PortType;
  /** Whether this wire has carried a value in the current run (SPEC §5.3). */
  live?: boolean;
}

/**
 * One wire.
 *
 * The curve comes from graph-core rather than from xyflow's own bezier helper,
 * because the control offset is a spec decision: `max(48, .55|Δx|, .22|Δy|)`
 * keeps a short wire from collapsing into a line and a long one from bowing
 * across a block (SPEC §5.1).
 *
 * A handle wire — `tools` or `memory` — is drawn heavier and carries a two-way
 * mark at the holder's end, because the call goes out and the reply comes back
 * on the same wire (SPEC §4.3). Everything else is a one-way flow.
 *
 * A wire that has carried a value this run animates: the dash travels from the
 * producer towards the consumer, so the direction is legible without reading
 * the graph (SPEC §5.3). Only flows animate — a handle never carries a value,
 * so a moving dash on one would be a lie about what the wire does.
 */
export function Wire({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  data,
  selected,
}: EdgeProps & { data?: WireData }) {
  const type = data?.type ?? 'any';
  const handle = type === 'tools' || type === 'memory';
  const live = !!data?.live && !handle;
  const colour = `var(--type-${type})`;

  const from = { x: sourceX, y: sourceY };
  const to = { x: targetX, y: targetY };
  const offset = controlOffset(targetX - sourceX, targetY - sourceY);
  const path = `M ${sourceX} ${sourceY} C ${sourceX + offset} ${sourceY}, ${targetX - offset} ${targetY}, ${targetX} ${targetY}`;

  return (
    <>
      {/* The halo sits under the core and is what makes a wire readable where
          it crosses another one. */}
      <BaseEdge
        id={`${id}-halo`}
        path={path}
        style={{
          stroke: colour,
          strokeOpacity: selected ? 0.28 : 0.09,
          strokeWidth: 5,
          fill: 'none',
        }}
      />
      <BaseEdge
        id={id}
        path={path}
        className={live ? s.live : undefined}
        style={{
          stroke: colour,
          strokeWidth: handle ? 2.2 : 1.9,
          fill: 'none',
        }}
      />
      {handle && <HandleMark from={from} to={to} colour={colour} />}
    </>
  );
}

/**
 * The two chevrons that say a wire is a handle. They sit at the holder's end —
 * the block that does the calling — so the direction of the call is visible
 * without selecting anything.
 */
function HandleMark({
  from,
  to,
  colour,
}: {
  from: { x: number; y: number };
  to: { x: number; y: number };
  colour: string;
}) {
  // 22px and 13px back from the target, per SPEC §16.3.
  const near = wirePoint(from, to, 0.86);
  const far = wirePoint(from, to, 0.78);
  return (
    <g stroke={colour} strokeWidth={1.4} fill="none" strokeLinecap="round" pointerEvents="none">
      <path d={`M ${far.x - 3} ${far.y - 3.5} L ${far.x + 1} ${far.y} L ${far.x - 3} ${far.y + 3.5}`} />
      <path
        d={`M ${near.x + 3} ${near.y - 3.5} L ${near.x - 1} ${near.y} L ${near.x + 3} ${near.y + 3.5}`}
      />
    </g>
  );
}
