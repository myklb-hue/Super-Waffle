import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { beforeEach, describe, expect, it } from 'vitest';
import { blockOfKind, emptyGraph, freshId, useDocument } from './document';
import type { Graph } from '@cyberloom/graph-core';

/**
 * A graph to edit. Read from the fixture through a crude parser rather than
 * built by hand, so these tests exercise the shapes the engine actually sends.
 */
function fixture(): Graph {
  const path = fileURLToPath(
    new URL('../../../../fixtures/graphs/customer-triage.loom', import.meta.url),
  );
  const text = readFileSync(path, 'utf8');
  const graph = emptyGraph();
  graph.id = 'customer-triage';
  graph.blocks = [...text.matchAll(/^ {2}- id: (.+)\n {4}kind: (.+)\n {4}position: \[(-?\d+), (-?\d+)\]/gm)].map(
    (m) => ({
      id: m[1]!,
      kind: m[2]!,
      title: null,
      position: { x: Number(m[3]), y: Number(m[4]) },
      size: null,
      view: 'summary',
      settings: {},
      ports: [],
      source: null,
      disabled: false,
      breakpoint: false,
      frame: null,
    }),
  );
  graph.wires = [...text.matchAll(/^ {4}from: (\S+)\.(\S+)\n {4}to: (\S+)\.(\S+)$/gm)].map(
    (m, i) => ({
      id: `w${i + 1}`,
      from: { node: m[1]!, port: m[2]! },
      to: { node: m[3]!, port: m[4]! },
    }),
  );
  return graph;
}

const doc = () => useDocument.getState();

beforeEach(() => {
  useDocument.getState().load('graphs/customer-triage.loom', fixture(), []);
});

describe('opening', () => {
  it('reads the fixture', () => {
    expect(doc().graph.blocks).toHaveLength(6);
    expect(doc().graph.wires).toHaveLength(5);
    expect(doc().dirty).toBe(false);
  });

  /** Otherwise the first undo after opening a file empties the document. */
  it('is not itself undoable', () => {
    expect(useDocument.temporal.getState().pastStates).toHaveLength(0);
  });
});

describe('moving a block', () => {
  it('snaps to the grid', () => {
    doc().moveBlocks(['input'], 5, -3);
    const input = doc().graph.blocks.find((b) => b.id === 'input')!;
    expect(input.position).toEqual({ x: 286, y: 154 });
    expect(doc().dirty).toBe(true);
  });

  it('moves a whole selection together', () => {
    doc().moveBlocks(['input', 'llm'], 22, 22);
    expect(doc().graph.blocks.find((b) => b.id === 'input')!.position).toEqual({
      x: 308,
      y: 176,
    });
    expect(doc().graph.blocks.find((b) => b.id === 'llm')!.position).toEqual({
      x: 836,
      y: 176,
    });
    // Everything else stays where it was.
    expect(doc().graph.blocks.find((b) => b.id === 'python')!.position).toEqual({
      x: 286,
      y: 616,
    });
  });
});

describe('wiring', () => {
  it('accepts a wire the grammar allows', () => {
    const before = doc().graph.wires.length;
    expect(
      doc().connect({ node: 'llm', port: 'text' }, { node: 'toolbox', port: 'pause' }),
    ).toBe(false); // text into an exec port
    expect(doc().connect({ node: 'terminal', port: 'stdout' }, { node: 'llm', port: 'prompt' })).toBe(
      true,
    ); // stream is accepted by text
    expect(doc().graph.wires).toHaveLength(before + 1);
  });

  /** The one place a wire can be refused: before it exists, not after. */
  it('refuses one the grammar does not', () => {
    const before = doc().graph.wires.length;
    expect(doc().connect({ node: 'terminal', port: 'tool' }, { node: 'llm', port: 'prompt' })).toBe(
      false,
    ); // tools into text
    expect(doc().connect({ node: 'llm', port: 'text' }, { node: 'nope', port: 'x' })).toBe(false);
    expect(doc().graph.wires).toHaveLength(before);
    expect(doc().dirty).toBe(false);
  });

  it('does not add the same wire twice', () => {
    const before = doc().graph.wires.length;
    expect(doc().connect({ node: 'input', port: 'value' }, { node: 'llm', port: 'prompt' })).toBe(
      false,
    );
    expect(doc().graph.wires).toHaveLength(before);
  });
});

describe('deleting', () => {
  it('takes a block and every wire that touched it', () => {
    doc().select({ kind: 'block', ids: ['toolbox'] });
    doc().deleteSelection();
    expect(doc().graph.blocks.find((b) => b.id === 'toolbox')).toBeUndefined();
    // w2, w3 and w4 all touched the toolbox.
    expect(doc().graph.wires.map((w) => w.id)).toEqual(['w1', 'w5']);
    expect(doc().selection).toEqual({ kind: 'none' });
  });

  it('takes a wire on its own without touching the blocks', () => {
    doc().select({ kind: 'wire', id: 'w1' });
    doc().deleteSelection();
    expect(doc().graph.wires).toHaveLength(4);
    expect(doc().graph.blocks).toHaveLength(6);
  });
});

describe('undo', () => {
  it('reverses an edit', () => {
    doc().moveBlocks(['input'], 44, 0);
    expect(doc().graph.blocks.find((b) => b.id === 'input')!.position.x).toBe(330);
    useDocument.temporal.getState().undo();
    expect(doc().graph.blocks.find((b) => b.id === 'input')!.position.x).toBe(286);
    useDocument.temporal.getState().redo();
    expect(doc().graph.blocks.find((b) => b.id === 'input')!.position.x).toBe(330);
  });

  it('covers a view change, which is an edit like any other', () => {
    doc().setBlockView('llm', 'compact');
    expect(doc().graph.blocks.find((b) => b.id === 'llm')!.view).toBe('compact');
    useDocument.temporal.getState().undo();
    expect(doc().graph.blocks.find((b) => b.id === 'llm')!.view).toBe('summary');
  });

  it('brings back a deleted block with its wires', () => {
    doc().select({ kind: 'block', ids: ['toolbox'] });
    doc().deleteSelection();
    useDocument.temporal.getState().undo();
    expect(doc().graph.blocks).toHaveLength(6);
    expect(doc().graph.wires).toHaveLength(5);
  });

  /**
   * Autosave runs between edits, and the engine sends back a canonical graph
   * that is a different object. If that counted as a step, one undo would land
   * on the moment just after the edit rather than before it, and every save
   * would cost an extra press.
   */
  it('is not confused by a save in the middle', () => {
    doc().moveBlocks(['input'], 44, 0);
    const steps = useDocument.temporal.getState().pastStates.length;
    // The engine replies with its own copy of the same graph.
    useDocument.getState().markSaved(structuredClone(doc().graph), []);
    expect(useDocument.temporal.getState().pastStates).toHaveLength(steps);
    useDocument.temporal.getState().undo();
    expect(doc().graph.blocks.find((b) => b.id === 'input')!.position.x).toBe(286);
  });

  /** Time travel restores the graph but knows nothing about `dirty`, so
   *  without `touch` an undone edit would never reach the file. */
  it('is written back to the file', () => {
    doc().moveBlocks(['input'], 44, 0);
    useDocument.getState().markSaved(doc().graph, []);
    expect(doc().dirty).toBe(false);
    useDocument.temporal.getState().undo();
    doc().touch();
    expect(doc().dirty).toBe(true);
  });

  /** Undo restores the graph, not the camera or what was selected. */
  it('leaves the selection alone', () => {
    doc().select({ kind: 'block', ids: ['llm'] });
    doc().moveBlocks(['llm'], 22, 0);
    useDocument.temporal.getState().undo();
    expect(doc().selection).toEqual({ kind: 'block', ids: ['llm'] });
  });
});

describe('settings', () => {
  it('sets and clears', () => {
    doc().setSetting('llm', 'model', 'qwen2.5:7b');
    expect(doc().graph.blocks.find((b) => b.id === 'llm')!.settings.model).toBe('qwen2.5:7b');
    // Clearing removes it, so the file says nothing where the user chose nothing.
    doc().setSetting('llm', 'model', '');
    expect(doc().graph.blocks.find((b) => b.id === 'llm')!.settings).not.toHaveProperty('model');
  });

  it('renames a block, and an empty name goes back to the kind', () => {
    doc().renameBlock('terminal', '  Thoughts  ');
    expect(doc().graph.blocks.find((b) => b.id === 'terminal')!.title).toBe('Thoughts');
    doc().renameBlock('terminal', '   ');
    expect(doc().graph.blocks.find((b) => b.id === 'terminal')!.title).toBeNull();
  });

  it('disables a mixed selection all at once, and a second press reverses it', () => {
    doc().toggleDisabled(['input', 'llm']);
    expect(doc().graph.blocks.filter((b) => b.disabled)).toHaveLength(2);
    doc().toggleDisabled(['input', 'llm']);
    expect(doc().graph.blocks.filter((b) => b.disabled)).toHaveLength(0);
  });
});

describe('adding a block', () => {
  it('gives it a readable id that does not collide', () => {
    expect(freshId(doc().graph, 'llm')).toBe('llm-2');
    const block = blockOfKind(doc().graph, 'llm', { x: 100, y: 205 });
    doc().addBlock(block);
    expect(block.id).toBe('llm-2');
    // Dropped position snaps to the grid.
    expect(block.position).toEqual({ x: 110, y: 198 });
    expect(doc().selection).toEqual({ kind: 'block', ids: ['llm-2'] });
  });

  it('gives a custom block somewhere to put its code', () => {
    const block = blockOfKind(doc().graph, 'custom', { x: 0, y: 0 });
    expect(block.source?.language).toBe('python');
    expect(block.source?.mode).toBe('inline');
  });
});
