import { describe, expect, it } from 'vitest';
import matrix from './generated/compatibility.json' with { type: 'json' };
import { PORT_TYPES, accepts, convertible, isClosed, refusal, targetsFor } from './compat';
import type { PortType } from './generated/schema';

type Pair = { from: PortType; to: PortType; accepted: boolean };
const PAIRS = matrix as Pair[];

describe('the type grammar', () => {
  /**
   * The test this file exists for. Rust decides the rule; this proves the copy
   * the drag uses has not drifted from it, for all one hundred ordered pairs.
   */
  it('agrees with the Rust implementation on every pair', () => {
    expect(PAIRS).toHaveLength(PORT_TYPES.length ** 2);
    const wrong = PAIRS.filter((p) => accepts(p.from, p.to) !== p.accepted).map(
      (p) => `${p.from} -> ${p.to}: Rust says ${p.accepted}, TypeScript says ${!p.accepted}`,
    );
    expect(wrong).toEqual([]);
  });

  it('covers every type in both directions', () => {
    for (const t of PORT_TYPES) {
      expect(PAIRS.filter((p) => p.from === t)).toHaveLength(PORT_TYPES.length);
      expect(PAIRS.filter((p) => p.to === t)).toHaveLength(PORT_TYPES.length);
    }
  });

  // The rows of SPEC §4.1, written out rather than derived, so a change to the
  // rule has to be an intentional edit here as well as in Rust.
  it('accepts a type into its own kind', () => {
    for (const t of PORT_TYPES) expect(accepts(t, t)).toBe(true);
  });

  it('lets a stream land on text or data', () => {
    expect(accepts('stream', 'text')).toBe(true);
    expect(accepts('stream', 'data')).toBe(true);
    expect(accepts('stream', 'image')).toBe(false);
  });

  it('refuses data into text, which is the pair the Convert block exists for', () => {
    expect(accepts('data', 'text')).toBe(false);
    expect(convertible('data', 'text')).toBe(true);
    expect(refusal('data', 'text')).toContain('Convert block');
  });

  it('keeps handles and triggers away from any', () => {
    for (const closed of ['tools', 'memory', 'exec'] as const) {
      expect(isClosed(closed)).toBe(true);
      expect(accepts(closed, 'any')).toBe(false);
      expect(accepts('any', closed)).toBe(false);
      expect(convertible(closed, 'text')).toBe(false);
    }
  });

  it('lets any reach every value port', () => {
    for (const t of PORT_TYPES.filter((t) => !isClosed(t))) {
      expect(accepts('any', t)).toBe(true);
      expect(accepts(t, 'any')).toBe(true);
    }
  });
});

describe('refusals', () => {
  it('says nothing when the wire is legal', () => {
    expect(refusal('text', 'text')).toBeNull();
    expect(refusal('stream', 'data')).toBeNull();
  });

  it('explains a handle in the words the tooltip uses', () => {
    expect(refusal('tools', 'text')).toContain('not a value');
    expect(refusal('exec', 'text')).toContain('trigger');
  });

  it('names both types when there is nothing to suggest', () => {
    const message = refusal('image', 'audio');
    expect(message).toContain('image');
    expect(message).toContain('audio');
  });
});

describe('targetsFor', () => {
  it('lists what an output can reach, which is what the drag dims', () => {
    expect(targetsFor('stream').sort()).toEqual(['any', 'data', 'stream', 'text']);
    expect(targetsFor('tools')).toEqual(['tools']);
    expect(targetsFor('exec')).toEqual(['exec']);
  });
});
