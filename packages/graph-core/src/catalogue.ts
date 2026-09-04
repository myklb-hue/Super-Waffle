/**
 * The block catalogue, as data.
 *
 * Rust owns it (`crates/block-kinds`); this reads the JSON that
 * `npm run gen` writes, so the library panel and the inspector work from the
 * same definitions the engine runs.
 */

import catalogue from './generated/catalogue.json' with { type: 'json' };
import type { BlockKind, Category, PortDef, Side } from './generated/schema';

export const KINDS = catalogue as unknown as BlockKind[];

const byId = new Map(KINDS.map((k) => [k.id, k]));

export function kind(id: string): BlockKind | undefined {
  return byId.get(id);
}

export function kindsIn(category: Category): BlockKind[] {
  return KINDS.filter((k) => k.category === category);
}

/** The shelves, in the order the library shows them (SPEC §6). */
export const CATEGORIES = [
  'models',
  'capabilities',
  'runtimes',
  'senses',
  'memory',
  'actuators',
  'data',
  'control',
  'human',
  'custom',
] as const satisfies readonly Category[];

export function portsOn(k: BlockKind, side: Side): PortDef[] {
  return k.ports.filter((p) => p.side === side);
}

/** A source keeps the graph armed, so a graph holding one never finishes. */
export function isSource(id: string): boolean {
  return kind(id)?.source ?? false;
}

/** Only a block with a picture offers a Stage view (SPEC §3.4). */
export function hasStage(id: string): boolean {
  return kind(id)?.stage ?? false;
}

/**
 * The third position of a block's view toggle, or null when it has none. A
 * block without a third view gets a two-position toggle rather than a greyed
 * out third (SPEC §3.4).
 */
export function thirdView(block: { kind: string }): 'code' | 'stage' | null {
  if (block.kind === 'custom') return 'code';
  return hasStage(block.kind) ? 'stage' : null;
}
