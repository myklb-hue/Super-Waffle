import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { CATEGORIES, KINDS, hasStage, isSource, kind, kindsIn, portsOn, thirdView } from './catalogue';
import { accepts } from './compat';
import type { PortType } from './generated/schema';

const fixturesDir = fileURLToPath(new URL('../../../fixtures/graphs', import.meta.url));

/**
 * The `.loom` reader lives in Rust; TypeScript only needs enough of one here to
 * check the catalogue against the fixtures without standing up the engine.
 * Deliberately crude: it reads the two lines a wire is written on and the kind
 * of each block, and nothing else.
 */
function readFixture(name: string) {
  const text = readFileSync(`${fixturesDir}/${name}`, 'utf8');
  const blocks = new Map<string, string>();
  let id: string | null = null;
  for (const line of text.split('\n')) {
    const block = line.match(/^ {2}- id: (.+)$/);
    if (block) id = block[1]!.replace(/^"|"$/g, '');
    const kindLine = line.match(/^ {4}kind: (.+)$/);
    if (kindLine && id) blocks.set(id, kindLine[1]!);
  }
  const wires = [...text.matchAll(/^ {4}from: (.+)\n {4}to: (.+)$/gm)].map((m) => ({
    from: m[1]!,
    to: m[2]!,
  }));
  return { blocks, wires };
}

const fixtures = readdirSync(fixturesDir).filter((f) => f.endsWith('.loom')).sort();

describe('the catalogue', () => {
  it('is the one Rust generated', () => {
    expect(KINDS.length).toBeGreaterThan(40);
    expect(kind('llm')?.category).toBe('models');
    expect(kind('nonexistent')).toBeUndefined();
  });

  it('has every shelf the library shows, and nothing else', () => {
    const used = new Set(KINDS.map((k) => k.category));
    expect([...used].sort()).toEqual([...CATEGORIES].sort());
    for (const c of CATEGORIES) expect(kindsIn(c).length).toBeGreaterThan(0);
  });

  it('carries the plain-words hints that state a boundary', () => {
    const webcam = kind('webcam')!;
    expect(webcam.settings.find((s) => s.name === 'store')?.hint).toContain(
      'never leave the machine',
    );
    const terminal = kind('terminal')!;
    expect(terminal.settings.find((s) => s.name === 'warnBefore')?.hint).toContain('Continue');
  });

  it('knows which blocks keep a graph armed', () => {
    expect(isSource('webcam')).toBe(true);
    expect(isSource('schedule')).toBe(true);
    expect(isSource('llm')).toBe(false);
  });

  it('gives a third view only to a block that has one', () => {
    expect(thirdView({ kind: 'custom' })).toBe('code');
    expect(thirdView({ kind: 'avatar' })).toBe('stage');
    expect(hasStage('avatar')).toBe(true);
    // A two-position toggle, not a greyed-out third.
    expect(thirdView({ kind: 'llm' })).toBeNull();
  });

  it('separates inputs from outputs', () => {
    const llm = kind('llm')!;
    expect(portsOn(llm, 'in').map((p) => p.name)).toContain('prompt');
    expect(portsOn(llm, 'out').map((p) => p.name)).toContain('text');
    expect(portsOn(llm, 'in').every((p) => p.side === 'in')).toBe(true);
  });
});

describe.each(fixtures)('%s', (name) => {
  const { blocks, wires } = readFixture(name);

  it('uses only kinds the catalogue has', () => {
    for (const [id, k] of blocks) {
      expect(k === 'custom' || kind(k) !== undefined, `${id} is a ${k}`).toBe(true);
    }
  });

  /**
   * The same check the Rust side makes, from the other language: every wire in
   * every fixture is legal under the rule this package implements. If the two
   * copies of the grammar ever disagree, one of them fails here.
   */
  it('has no wire the type grammar refuses', () => {
    for (const wire of wires) {
      const from = portTypeOf(wire.from, 'out');
      const to = portTypeOf(wire.to, 'in');
      if (from === null || to === null) continue; // a custom block or a frame
      expect(accepts(from, to), `${wire.from} -> ${wire.to}`).toBe(true);
    }
  });

  function portTypeOf(ref: string, side: 'in' | 'out'): PortType | null {
    const [node, portName] = ref.split('.');
    const k = blocks.get(node!);
    if (!k || k === 'custom') return null;
    const def = kind(k);
    if (!def) return null;
    return def.ports.find((p) => p.name === portName && p.side === side)?.type ?? null;
  }
});
