import { describe, expect, it } from 'vitest';
import { aspectOf, expressionsOf, rigIds, stateOf, useRigs } from './rigs';

describe('the rigs the shell can draw (SPEC §11.1)', () => {
  it('starts with the four that ship, in name order', () => {
    expect(rigIds()).toEqual(['line', 'robot', 'orb', 'pixel']);
    expect(stateOf('line', 'smile')).toContain('<svg');
    // A face a rig cannot make falls back to neutral rather than to nothing.
    expect(stateOf('pixel', 'sleepy')).toBe(stateOf('pixel', 'neutral'));
  });

  it('lists expressions in the panel order, with anything extra after', () => {
    expect(expressionsOf('line')).toEqual([
      'neutral',
      'smile',
      'frown',
      'surprised',
      'thinking',
      'speaking',
      'love',
      'sleepy',
    ]);
    expect(expressionsOf('x', { zzz: '<svg/>', neutral: '<svg/>', aaa: '<svg/>' })).toEqual([
      'neutral',
      'aaa',
      'zzz',
    ]);
  });

  it('learns a workspace rig from the engine, and a workspace rig with a shipped name replaces it', () => {
    const before = stateOf('line', 'neutral');
    useRigs.getState().learn([
      {
        id: 'moon',
        name: 'Moon',
        description: 'A crescent',
        expressions: ['neutral', 'smile'],
        gestures: [],
        gaze: false,
        blinkMs: 6000,
        breathePerMin: 0,
        shipped: false,
        states: { neutral: '<svg viewBox="0 0 300 200"><g id="eyes"/></svg>', smile: '<svg/>' },
      },
      {
        id: 'line',
        name: 'Line',
        description: 'Mine',
        expressions: ['neutral'],
        gestures: ['nod'],
        gaze: true,
        blinkMs: 4000,
        breathePerMin: 13,
        shipped: false,
        states: { neutral: '<svg><!-- mine --></svg>' },
      },
    ]);
    expect(rigIds()).toEqual(['line', 'robot', 'orb', 'pixel', 'moon']);
    expect(stateOf('moon', 'smile')).toBe('<svg/>');
    expect(stateOf('line', 'neutral')).toContain('mine');
    expect(stateOf('line', 'neutral')).not.toBe(before);
    expect(useRigs.getState().rigs.line!.shipped).toBe(false);
  });

  it('reads the aspect from the drawing, and is square when there is nothing to read', () => {
    expect(aspectOf({ neutral: '<svg viewBox="0 0 300 200"/>' })).toBeCloseTo(1.5);
    expect(aspectOf({ neutral: '<svg viewBox="0 0 240 240"/>' })).toBe(1);
    expect(aspectOf({})).toBe(1);
    expect(aspectOf({ neutral: '<svg/>' })).toBe(1);
  });
});
