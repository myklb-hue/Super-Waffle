/**
 * The rigs the shell can draw (SPEC §11.1).
 *
 * The four reference rigs are bundled: they ship with the application, and a
 * face that has to be fetched before it can blink is a face that arrives late.
 * The SVG is taken as text rather than as a URL because the animation reaches
 * inside it — `#eyes` blinks and moves with the gaze, `#mouth` opens with the
 * speech — and an `<img>` is a closed box.
 *
 * A rig the *user* added lives in their workspace and is not here. Drawing one
 * needs the engine to hand its states over, which is a door this slice has not
 * opened: the vocabulary is already generated from whatever rig the engine
 * loaded, so the model side works today and only the picture would be missing.
 */
const files = import.meta.glob('../../../../rigs/*/states/*.svg', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

export type RigStates = Record<string, string>;

const byRig: Record<string, RigStates> = {};
for (const [path, svg] of Object.entries(files)) {
  const match = /rigs\/([^/]+)\/states\/([^/]+)\.svg$/.exec(path);
  if (!match) continue;
  const [, rig, state] = match;
  if (!rig || !state) continue;
  (byRig[rig] ??= {})[state] = svg;
}

/** Every rig the shell has, in name order. */
export const RIGS = Object.keys(byRig).sort();

/** The states of one rig, or an empty set for a rig only the engine knows. */
export function statesOf(rig: string): RigStates {
  return byRig[rig] ?? {};
}

/** What a rig can wear, in the order the panel lists them. */
const ORDER = ['neutral', 'smile', 'frown', 'surprised', 'thinking', 'speaking', 'love', 'sleepy'];

export function expressionsOf(rig: string): string[] {
  const has = statesOf(rig);
  return ORDER.filter((e) => e in has).concat(
    Object.keys(has)
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
