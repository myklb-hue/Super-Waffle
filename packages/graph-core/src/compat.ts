/**
 * The one rule that decides whether a wire is legal (SPEC §4.1).
 *
 * This is a second implementation of `PortType::accepted_by` in
 * `crates/graph-format/src/types.rs`. It exists because the drag needs an
 * answer per frame, in the browser, with no round trip to the engine; and it
 * cannot be allowed to drift, so the Rust side exports every ordered pair to
 * `generated/compatibility.json` and `compat.test.ts` checks this file against
 * it. Change the rule in Rust, regenerate, and the test tells you to change it
 * here too.
 */

import type { PortType } from './generated/schema';

export const PORT_TYPES = [
  'text',
  'tools',
  'memory',
  'data',
  'stream',
  'image',
  'audio',
  'file',
  'exec',
  'any',
] as const satisfies readonly PortType[];

/**
 * A closed type carries something that is not a value: a handle the holder
 * calls (`tools`, `memory`) or control flow (`exec`). Closed types connect only
 * to a port of the same type, so an `any` port does not accept them even though
 * `any` "accepts everything" — those are not values.
 */
export function isClosed(type: PortType): boolean {
  return type === 'tools' || type === 'memory' || type === 'exec';
}

/** Whether a wire from a `from` output may land on a `to` input. */
export function accepts(from: PortType, to: PortType): boolean {
  if (isClosed(from) || isClosed(to)) return from === to;
  if (from === 'any' || to === 'any') return true;
  // A stream of text is text; a stream of records is data. It is also still a
  // stream, so this cannot short-circuit the identity case below.
  if (from === 'stream' && (to === 'text' || to === 'data')) return true;
  return from === to;
}

/**
 * Whether the two are close enough that the shell should offer to insert a
 * Convert block rather than simply refusing the drag (SPEC §5.5, §15.5).
 *
 * Deliberately narrow: a conversion has to be one a reasonable person would
 * expect to succeed. Anything wider belongs to a block the user chooses.
 */
export function convertible(from: PortType, to: PortType): boolean {
  if (accepts(from, to)) return false;
  if (isClosed(from) || isClosed(to)) return false;
  const pairs: ReadonlyArray<readonly [PortType, PortType]> = [
    ['data', 'text'],
    ['text', 'data'],
    ['file', 'image'],
    ['file', 'audio'],
    ['file', 'text'],
    ['image', 'file'],
    ['audio', 'file'],
  ];
  return pairs.some(([a, b]) => a === from && b === to);
}

/** Why a drag was refused, in the words the tooltip uses. */
export function refusal(from: PortType, to: PortType): string | null {
  if (accepts(from, to)) return null;
  if (isClosed(from) && from !== to) {
    const what =
      from === 'exec' ? 'a trigger, not a value' : `a ${from} handle, not a value`;
    return `${from} is ${what}; it only meets another ${from} port`;
  }
  if (isClosed(to) && from !== to) {
    return `${to} takes only a ${to} port`;
  }
  if (convertible(from, to)) {
    return `${from} does not fit ${to}. Insert a Convert block?`;
  }
  return `${from} is not accepted by ${to}`;
}

/** The types a given output could legally reach. Used to dim the rest mid-drag. */
export function targetsFor(from: PortType): PortType[] {
  return PORT_TYPES.filter((t) => accepts(from, t));
}
