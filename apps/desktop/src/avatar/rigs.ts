/**
 * The rigs the shell can draw (SPEC §11.1).
 *
 * The four reference rigs are bundled: they ship with the application, and a
 * face that has to be fetched before it can blink is a face that arrives late.
 * The SVG is taken as text rather than as a URL because the animation reaches
 * inside it — `#eyes` blinks and moves with the gaze, `#mouth` opens with the
 * speech — and an `<img>` is a closed box.
 *
 * A rig the *user* added lives in their workspace and is not bundled. The
 * engine hands its states over (`workspace.rigs`), and `refreshRigs` folds
 * them in here: a workspace rig with a shipped rig's name replaces it, which
 * is the precedence the engine itself uses when it dresses the block. Until
 * that answer arrives the bundled four are what there is, so the picker never
 * opens empty.
 */
import { create } from 'zustand';
import type { RigInfo } from '@cyberloom/graph-core';
import { listRigs } from '../stores/rpc';

const files = import.meta.glob('../../../../rigs/*/states/*.svg', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

export type RigStates = Record<string, string>;

/** What the shell knows about one rig. The bundled four know less than the
 *  engine says, and are filled in when it answers. */
export interface KnownRig {
  id: string;
  name: string;
  states: RigStates;
  gestures: string[];
  gaze: boolean;
  shipped: boolean;
}

/** What a rig can wear, in the order the panel lists them. */
const ORDER = ['neutral', 'smile', 'frown', 'surprised', 'thinking', 'speaking', 'love', 'sleepy'];

function bundled(): Record<string, KnownRig> {
  const byRig: Record<string, KnownRig> = {};
  for (const [path, svg] of Object.entries(files)) {
    const match = /rigs\/([^/]+)\/states\/([^/]+)\.svg$/.exec(path);
    if (!match) continue;
    const [, rig, state] = match;
    if (!rig || !state) continue;
    (byRig[rig] ??= {
      id: rig,
      name: rig.charAt(0).toUpperCase() + rig.slice(1),
      states: {},
      gestures: [],
      gaze: true,
      shipped: true,
    }).states[state] = svg;
  }
  return byRig;
}

interface RigStore {
  rigs: Record<string, KnownRig>;
  /** Fold in what the engine says it has. */
  learn: (from: RigInfo[]) => void;
}

export const useRigs = create<RigStore>()((set) => ({
  rigs: bundled(),
  learn: (from) =>
    set((st) => {
      const rigs = { ...st.rigs };
      for (const info of from) {
        rigs[info.id] = {
          id: info.id,
          name: info.name,
          states: info.states,
          gestures: info.gestures,
          gaze: info.gaze,
          shipped: info.shipped,
        };
      }
      return { rigs };
    }),
}));

/** Ask the engine what it can wear. Safe to call more than once; a failure
 *  leaves the bundled rigs in place, which is a face rather than an error. */
export async function refreshRigs(): Promise<void> {
  try {
    useRigs.getState().learn(await listRigs());
  } catch {
    // The engine may not be up yet, or may be older than this shell. The four
    // bundled rigs are still here either way.
  }
}

/** The four that ship, in the order Figure 15 shows them. */
const SHIPPED_ORDER = ['line', 'robot', 'orb', 'pixel'];

/** Every rig the shell has: the shipped four in their order, then the
 *  workspace's own, by name. */
export function rigIds(rigs: Record<string, KnownRig> = useRigs.getState().rigs): string[] {
  const shipped = SHIPPED_ORDER.filter((id) => id in rigs);
  const own = Object.keys(rigs)
    .filter((id) => !SHIPPED_ORDER.includes(id))
    .sort();
  return [...shipped, ...own];
}

/** The states of one rig, or an empty set for a rig nobody has. */
export function statesOf(rig: string): RigStates {
  return useRigs.getState().rigs[rig]?.states ?? {};
}

export function expressionsOf(rig: string, states: RigStates = statesOf(rig)): string[] {
  return ORDER.filter((e) => e in states).concat(
    Object.keys(states)
      .filter((e) => !ORDER.includes(e))
      .sort(),
  );
}

/**
 * The markup for one expression, falling back to neutral.
 *
 * A rig that cannot make a face is not an error here: the engine already
 * refuses the tool call, and if something does get through, a neutral face is a
 * better answer than an empty block.
 */
export function stateOf(rig: string, expression: string): string | undefined {
  const states = statesOf(rig);
  return states[expression] ?? states.neutral;
}

/**
 * Width over height, from the rig's own drawing.
 *
 * The four that ship are square; a user's need not be, and "an Avatar keeps
 * its rig's aspect ratio" (SPEC §3.4) means the rig's, not ours. Read from the
 * `viewBox` of the resting face, and 1 when there is nothing to read.
 */
export function aspectOf(states: RigStates): number {
  const svg = states.neutral ?? Object.values(states)[0];
  const box = svg ? /viewBox="\s*[-\d.]+\s+[-\d.]+\s+([\d.]+)\s+([\d.]+)\s*"/.exec(svg) : null;
  if (!box) return 1;
  const w = Number(box[1]);
  const h = Number(box[2]);
  return w > 0 && h > 0 ? w / h : 1;
}
