/**
 * Where things are on the canvas.
 *
 * These numbers are load-bearing and they are not free-floating: they mirror
 * the geometry tokens in `packages/ui/src/styles/tokens.css`, which the
 * artboards are generated from. `geometry.test.ts` reads that file and checks
 * the two agree, so a token cannot be changed without this following.
 */

import type { Block, Frame, Port, Position } from './generated/schema';

/** The block header, and so where the first port sits. */
export const BLOCK_HEADER = 32;
export const BLOCK_HEADER_STAGE = 24;
/** One row per port index. */
export const PORT_ROW = 24;
/** The centre of row 0, measured from the block's top. */
export const PORT_FIRST = 52;
export const PORT_DOT = 11;
export const BLOCK_MIN_WIDTH = 168;
export const GRID = 22;

/**
 * The centre of port row `i`, from the block's top. This single expression is
 * what the wire router and the block renderer both use; if they ever compute it
 * separately, wires stop landing on dots.
 */
export function portY(index: number, view: Block['view'] = 'summary'): number {
  const header = view === 'stage' ? BLOCK_HEADER_STAGE : BLOCK_HEADER;
  return header + (PORT_FIRST - BLOCK_HEADER) + index * PORT_ROW;
}

/** Where a port's dot sits in canvas coordinates. */
export function portPoint(
  block: { position: Position; size?: { w: number } | null; view: Block['view'] },
  index: number,
  side: 'in' | 'out',
  width = block.size?.w ?? BLOCK_MIN_WIDTH,
): Position {
  return {
    x: block.position.x + (side === 'in' ? 0 : width),
    y: block.position.y + portY(index, block.view),
  };
}

/**
 * How far the bezier's control points reach horizontally.
 *
 * The floor of 48 keeps a short wire from collapsing into a straight line; the
 * two ratios keep a long or steep one from bowing so far it crosses a block.
 */
export function controlOffset(dx: number, dy: number): number {
  return Math.max(48, 0.55 * Math.abs(dx), 0.22 * Math.abs(dy));
}

/** The SVG path for a wire between two points. */
export function wirePath(from: Position, to: Position): string {
  const offset = controlOffset(to.x - from.x, to.y - from.y);
  return `M ${from.x} ${from.y} C ${from.x + offset} ${from.y}, ${to.x - offset} ${to.y}, ${to.x} ${to.y}`;
}

/** A point on that curve, for placing the two-way mark on a handle wire. */
export function wirePoint(from: Position, to: Position, t: number): Position {
  const offset = controlOffset(to.x - from.x, to.y - from.y);
  const c1 = { x: from.x + offset, y: from.y };
  const c2 = { x: to.x - offset, y: to.y };
  const u = 1 - t;
  const a = u * u * u;
  const b = 3 * u * u * t;
  const c = 3 * u * t * t;
  const d = t * t * t;
  return {
    x: a * from.x + b * c1.x + c * c2.x + d * to.x,
    y: a * from.y + b * c1.y + c * c2.y + d * to.y,
  };
}

/** How tall a block is, given how many rows of ports it shows. */
export function blockHeight(
  ports: { in: number; out: number },
  view: Block['view'],
  bodyHeight = 0,
): number {
  const header = view === 'stage' ? BLOCK_HEADER_STAGE : BLOCK_HEADER;
  if (view === 'compact') {
    return header + Math.max(ports.in, ports.out) * PORT_ROW + 8;
  }
  return header + Math.max(ports.in, ports.out) * PORT_ROW + bodyHeight + 8;
}

/** Positions are rounded to the grid on save, so a nudge leaves no diff. */
export function snap(p: Position): Position {
  return { x: Math.round(p.x / GRID) * GRID, y: Math.round(p.y / GRID) * GRID };
}

/** Which ports a block shows: an unwired optional port stays hidden (SPEC §4.5). */
export function visiblePorts(
  ports: readonly Port[],
  wired: ReadonlySet<string>,
  selected = false,
): Port[] {
  return ports.filter((p) => !p.optional || selected || wired.has(p.name));
}

/** The rectangle a loop frame covers. */
export function frameBounds(frame: Frame): {
  x: number;
  y: number;
  w: number;
  h: number;
} {
  return {
    x: frame.position.x,
    y: frame.position.y,
    w: frame.size.w,
    h: frame.size.h ?? 0,
  };
}

/** Whether a block sits inside a frame's rectangle, which is how a drop decides. */
export function isInside(block: Position, frame: Frame): boolean {
  const b = frameBounds(frame);
  return block.x >= b.x && block.y >= b.y && block.x <= b.x + b.w && block.y <= b.y + b.h;
}
