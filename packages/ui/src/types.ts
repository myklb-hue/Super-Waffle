// The vocabulary every primitive shares. Nothing here knows about graphs,
// blocks or the engine; those types live in @cyberloom/graph-core.

/** The ten port types. A wire is coloured by its type, and the type is what
 *  makes a connection legal or refuses it (SPEC 4.1). */
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
] as const;

export type PortType = (typeof PORT_TYPES)[number];

/** The ten block categories. A category carries its own colour, shared
 *  between the library shelf and the block header (SPEC 6). */
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
] as const;

export type Category = (typeof CATEGORIES)[number];

/** What a block or a connection is doing right now (SPEC §3.2). */
export const STATUS_STATES = [
  'idle',
  'queued',
  'running',
  'ok',
  'error',
  'off',
  /**
   * Bound as a capability, waiting to be called.
   *
   * Not one of §3.2's seven, because those describe blocks that execute. A
   * Terminal offered to a model through a Toolbox never runs on its own, and
   * showing it as `ok` would claim it had produced a value when it had not
   * been asked for one. Drawn as an unfilled ring: available, not finished.
   */
  'ready',
] as const;

export type StatusState = (typeof STATUS_STATES)[number];

/** How much of a block is drawn. Every block has compact and summary; a
 *  custom block's third view is its code, a block with a picture is its
 *  stage (SPEC 3.4). */
export type View = 'compact' | 'summary' | 'code' | 'stage';

/** The third view a block offers, if it offers one. */
export type ThirdView = 'code' | 'stage' | null;

/**
 * A colour named by token rather than by value, so no component writes a
 * literal. Resolve with `colorVar`.
 */
export type ColorToken =
  | 'accent'
  | 'ok'
  | 'warn'
  | 'err'
  | 'text-hi'
  | 'text-body'
  | 'text-mid'
  | 'text-low'
  | 'text-faint'
  | `type-${PortType}`
  | `cat-${Category}`;

/** The CSS custom property a colour token resolves to. */
export function colorVar(token: ColorToken | undefined, fallback = 'currentColor'): string {
  return token ? `var(--${token})` : fallback;
}

/** A tint of a colour token, for chip fills, wells and halos. The alpha
 *  values themselves are tokens (--a-chip and friends) so a tint is never a
 *  hard-coded rgba. */
export function tint(token: ColorToken, alpha: `--a-${string}` | string): string {
  return `color-mix(in srgb, var(--${token}) ${
    alpha.startsWith('--') ? `var(${alpha})` : alpha
  }, transparent)`;
}
