import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import {
  BLOCK_HEADER,
  BLOCK_HEADER_STAGE,
  BLOCK_MIN_WIDTH,
  GRID,
  PORT_DOT,
  PORT_FIRST,
  PORT_ROW,
  controlOffset,
  isInside,
  portPoint,
  portY,
  snap,
  visiblePorts,
  wirePath,
  wirePoint,
} from './geometry';
import type { Frame, Port } from './generated/schema';

const tokens = readFileSync(
  fileURLToPath(new URL('../../ui/src/styles/tokens.css', import.meta.url)),
  'utf8',
);

function token(name: string): number {
  const match = tokens.match(new RegExp(`--${name}:\\s*(-?[\\d.]+)px`));
  if (!match) throw new Error(`--${name} is not in tokens.css`);
  return Number(match[1]);
}

describe('geometry follows the tokens', () => {
  /**
   * The numbers here and the numbers the artboards are drawn from have to be
   * the same numbers, or wires stop landing on dots. tokens.css is the
   * authority; this test is the link.
   */
  it.each([
    ['block-header-h', BLOCK_HEADER],
    ['block-header-stage-h', BLOCK_HEADER_STAGE],
    ['port-row', PORT_ROW],
    ['port-first', PORT_FIRST],
    ['port-dot', PORT_DOT],
    ['block-min-w', BLOCK_MIN_WIDTH],
    ['grid', GRID],
  ])('%s matches tokens.css', (name, value) => {
    expect(value).toBe(token(name));
  });
});

describe('port rows', () => {
  it('centres row i at 52 + 24i, which is what the wire router uses', () => {
    expect(portY(0)).toBe(52);
    expect(portY(1)).toBe(76);
    expect(portY(2)).toBe(100);
    expect(portY(5)).toBe(172);
  });

  it('keeps the rows where they were when the header shrinks in Stage view', () => {
    // The header is 8px shorter, so every row moves up by 8 and no more: ports
    // are dots on the edges at the same spacing (SPEC §3.4).
    expect(portY(0, 'stage')).toBe(portY(0) - (BLOCK_HEADER - BLOCK_HEADER_STAGE));
    expect(portY(3, 'stage') - portY(2, 'stage')).toBe(PORT_ROW);
  });

  it('puts inputs on the left edge and outputs on the right', () => {
    const block = { position: { x: 100, y: 200 }, size: { w: 220, h: null }, view: 'summary' as const };
    expect(portPoint(block, 0, 'in')).toEqual({ x: 100, y: 252 });
    expect(portPoint(block, 0, 'out')).toEqual({ x: 320, y: 252 });
    expect(portPoint(block, 2, 'out')).toEqual({ x: 320, y: 300 });
  });

  it('falls back to the minimum width when a block has no size of its own', () => {
    const block = { position: { x: 0, y: 0 }, view: 'summary' as const };
    expect(portPoint(block, 0, 'out').x).toBe(BLOCK_MIN_WIDTH);
  });
});

describe('wires', () => {
  it('never lets a short wire collapse into a straight line', () => {
    expect(controlOffset(0, 0)).toBe(48);
    expect(controlOffset(10, 10)).toBe(48);
  });

  it('reaches further as the wire gets longer or steeper', () => {
    expect(controlOffset(400, 0)).toBeCloseTo(220);
    expect(controlOffset(0, 400)).toBeCloseTo(88);
    // Whichever term is larger wins.
    expect(controlOffset(400, 1200)).toBeCloseTo(264);
  });

  it('is symmetric: dragging right to left bows the same amount', () => {
    expect(controlOffset(-400, 0)).toBe(controlOffset(400, 0));
    expect(controlOffset(0, -400)).toBe(controlOffset(0, 400));
  });

  it('starts and ends exactly on the two dots', () => {
    const from = { x: 100, y: 200 };
    const to = { x: 500, y: 340 };
    expect(wirePath(from, to)).toMatch(/^M 100 200 C /);
    expect(wirePath(from, to)).toMatch(/500 340$/);
    expect(wirePoint(from, to, 0)).toEqual(from);
    expect(wirePoint(from, to, 1)).toEqual(to);
  });

  it('finds the midpoint, which is where a handle mark sits', () => {
    const mid = wirePoint({ x: 0, y: 0 }, { x: 400, y: 200 }, 0.5);
    expect(mid.x).toBeCloseTo(200);
    expect(mid.y).toBeCloseTo(100);
  });
});

describe('the grid', () => {
  it('snaps a nudge back to where it was', () => {
    expect(snap({ x: 286 + 3, y: 154 - 4 })).toEqual({ x: 286, y: 154 });
  });

  it('rounds rather than floors, so a block does not creep upward', () => {
    expect(snap({ x: 12, y: 11 })).toEqual({ x: 22, y: 22 });
    expect(snap({ x: 10, y: 10 })).toEqual({ x: 0, y: 0 });
  });
});

describe('optional ports', () => {
  const ports: Port[] = [
    { name: 'prompt', type: 'text', side: 'in', optional: false },
    { name: 'context', type: 'data', side: 'in', optional: true },
    { name: 'tools', type: 'tools', side: 'in', optional: true },
  ];

  it('hides an unwired optional port', () => {
    expect(visiblePorts(ports, new Set()).map((p) => p.name)).toEqual(['prompt']);
  });

  it('shows one as soon as it is wired', () => {
    expect(visiblePorts(ports, new Set(['tools'])).map((p) => p.name)).toEqual([
      'prompt',
      'tools',
    ]);
  });

  it('shows all of them while the block is selected, so they can be wired', () => {
    expect(visiblePorts(ports, new Set(), true)).toHaveLength(3);
  });
});

describe('loop frames', () => {
  const frame: Frame = {
    id: 'loop',
    kind: 'loop',
    position: { x: 100, y: 100 },
    size: { w: 400, h: 200 },
    over: { node: 'watch', port: 'file' },
    as: 'item',
    parallel: 2,
    max: 100,
    stopWhen: null,
    continueOnError: true,
  };

  it('knows which blocks are inside it', () => {
    expect(isInside({ x: 200, y: 150 }, frame)).toBe(true);
    expect(isInside({ x: 100, y: 100 }, frame)).toBe(true);
    expect(isInside({ x: 500, y: 300 }, frame)).toBe(true);
    expect(isInside({ x: 501, y: 150 }, frame)).toBe(false);
    expect(isInside({ x: 200, y: 99 }, frame)).toBe(false);
  });
});
