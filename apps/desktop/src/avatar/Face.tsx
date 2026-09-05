import { useEffect, useRef, useState } from 'react';
import { useRigs } from './rigs';
import s from './Face.module.css';

/**
 * The avatar, drawn (SPEC §11.3, §11.4).
 *
 * Three channels, none of them waiting for the others: the expression is what
 * the model or an `express` wire asked for, the mouth is the shape of the audio
 * that is playing, the gaze is wherever `look` pointed. Each has its own clock
 * here, exactly as it does in the engine.
 *
 * Idle is the block's own (§11.4). The blink, the breath and the drift of the
 * gaze happen here, with nothing running, because an avatar that only moves
 * when the graph does is a picture rather than a presence. The settle back to
 * neutral and the sleep are the engine's, because they change what the face
 * *is*, and the `state` port has to say the same thing the window shows. The
 * numbers — how often to blink, how fast to breathe — come from the engine
 * too, read from the rig and the block, so the two never disagree about them.
 */
export function Face({
  rig,
  expression = 'neutral',
  intensity = 1,
  mouth,
  gaze,
  gazeAt,
  gesture,
  gestureSeq = 0,
  asleep = false,
  blinkMs = 4000,
  breathePerMin = 13,
  idle = true,
  size = 200,
}: {
  rig: string;
  expression?: string;
  intensity?: number;
  /** Loudness over time, 0–255 a bucket, twelve buckets a second. */
  mouth?: number[];
  gaze?: string | null;
  /** Where the gaze is, `0..1` from the top left, when it came with a place. */
  gazeAt?: [number, number] | null;
  /** A one-shot to play; `gestureSeq` changing is what plays it again. */
  gesture?: string | null;
  gestureSeq?: number;
  asleep?: boolean;
  blinkMs?: number;
  /** Zero is a face that does not breathe. */
  breathePerMin?: number;
  idle?: boolean;
  size?: number;
}) {
  const states = useRigs((r) => r.rigs[rig]?.states);
  const svg = states?.[expression] ?? states?.neutral;
  const talkingMouth = mouthOf(states?.speaking);
  const blinking = useBlink(idle && !asleep, blinkMs);
  const openness = useMouth(mouth);
  const drift = useDrift(idle && !asleep && !gaze && !gazeAt);
  const playing = useGesture(gesture, gestureSeq);
  const body = useRef<HTMLDivElement>(null);
  const speaking = openness > 0.12;

  // The mouth that talks is the one the rig's author drew for talking. The
  // `speaking` state differs from the others only in its mouth, so that group
  // is swapped in while the envelope is open and the eyes stay whatever the
  // expression made them — a smile keeps smiling while it speaks. Done to the
  // DOM rather than by re-rendering the SVG, because React owns the markup as
  // one string and would redraw the whole face on every syllable.
  useEffect(() => {
    const group = body.current?.querySelector<SVGGElement>('#mouth');
    if (!group) return;
    if (speaking && talkingMouth && group.innerHTML !== talkingMouth) {
      const own = group.innerHTML;
      group.innerHTML = talkingMouth;
      return () => {
        group.innerHTML = own;
      };
    }
    return undefined;
  }, [svg, speaking, talkingMouth]);

  if (!svg) {
    return (
      <div className={s.missing} style={{ width: size, height: size }}>
        {rig} has no face to draw here
      </div>
    );
  }

  const scale = size / 200;
  // A place beats a name. A name is nudged deterministically from its letters,
  // so the same person recognised twice in a row does not make the face
  // twitch; the idle drift is a third, smaller thing that only happens when
  // nothing is being looked at.
  const look = gazeAt
    ? { x: (gazeAt[0] - 0.5) * 18, y: (gazeAt[1] - 0.5) * 10 }
    : gaze
      ? offsetFor(gaze)
      : drift;
  const breathes = idle && !asleep && breathePerMin > 0;

  return (
    <div
      className={`${s.face} ${breathes ? s.alive : ''} ${asleep ? s.asleep : ''}`}
      style={{
        width: size,
        height: size,
        // Intensity dims the whole expression rather than redrawing it: a smile
        // at 0.2 is the same smile, held back. Sleep dims further.
        ['--intensity' as string]: String((asleep ? 0.55 : 1) * (0.45 + 0.55 * clamp(intensity))),
        ['--breath' as string]: `${breathePerMin > 0 ? 60 / breathePerMin : 4.6}s`,
        // With the talking mouth in place the envelope only has to widen it a
        // little; a rig with no talking mouth gets the whole opening from the
        // scale.
        ['--mouth' as string]: String(openness * (talkingMouth ? 0.35 : 1.15)),
        ['--gaze-x' as string]: `${look.x * scale}px`,
        ['--gaze-y' as string]: `${look.y * scale}px`,
        ['--blink' as string]: asleep ? '0.15' : blinking ? '0.06' : '1',
      }}
      data-testid={`face-${rig}`}
      data-expression={expression}
      data-speaking={speaking ? 'yes' : 'no'}
      data-asleep={asleep ? 'yes' : 'no'}
      data-gesture={playing ?? undefined}
    >
      <div
        ref={body}
        className={`${s.body} ${playing === 'nod' ? s.nod : ''} ${playing === 'shake' ? s.shake : ''}`}
        dangerouslySetInnerHTML={{ __html: svg }}
      />
    </div>
  );
}

/** The inside of a state's `#mouth` group, or nothing when it has none. */
function mouthOf(svg: string | undefined): string | null {
  if (!svg) return null;
  const match = /<g id="mouth"[^>]*>([\s\S]*?)<\/g>/.exec(svg);
  return match?.[1] ?? null;
}

/**
 * The blink. A cadence rather than a metronome: a face that blinks exactly
 * every four seconds reads as a machine pretending, which is worse than a
 * machine. `every` is the rig's number, or the block's; the jitter is ours.
 */
function useBlink(on: boolean, every: number): boolean {
  const [blinking, setBlinking] = useState(false);
  useEffect(() => {
    if (!on || every <= 0) {
      setBlinking(false);
      return;
    }
    let alive = true;
    let timer: ReturnType<typeof setTimeout>;
    const again = () => {
      timer = setTimeout(
        () => {
          if (!alive) return;
          setBlinking(true);
          setTimeout(() => alive && setBlinking(false), 120);
          again();
        },
        every * (0.65 + Math.random() * 0.8),
      );
    };
    again();
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [on, every]);
  return blinking;
}

/**
 * Play an envelope back at the rate it was sampled at.
 *
 * The engine sends twelve buckets a second because that is a mouth moving with
 * the syllables. Playing them back on a timer rather than tying them to audio
 * playback keeps the mouth honest when there is no sound device — the shape
 * came from the real audio either way.
 */
function useMouth(envelope: number[] | undefined): number {
  const [open, setOpen] = useState(0);
  const at = useRef(0);

  useEffect(() => {
    if (!envelope || envelope.length === 0) {
      setOpen(0);
      return;
    }
    at.current = 0;
    const timer = setInterval(() => {
      const value = envelope[at.current];
      if (value === undefined) {
        clearInterval(timer);
        setOpen(0);
        return;
      }
      setOpen(value / 255);
      at.current += 1;
    }, 1000 / 12);
    return () => clearInterval(timer);
  }, [envelope]);

  return open;
}

/**
 * Gaze drift (SPEC §11.4): with nothing to look at, the eyes wander a little
 * and come back, on their own slow clock. Small on purpose — a glance, not a
 * search — and never while something *is* being looked at.
 */
function useDrift(on: boolean): { x: number; y: number } {
  const [at, setAt] = useState({ x: 0, y: 0 });
  useEffect(() => {
    if (!on) {
      setAt({ x: 0, y: 0 });
      return;
    }
    let alive = true;
    let timer: ReturnType<typeof setTimeout>;
    const again = () => {
      timer = setTimeout(
        () => {
          if (!alive) return;
          // Half the time it comes home; the rest, somewhere near.
          setAt(
            Math.random() < 0.5
              ? { x: 0, y: 0 }
              : { x: (Math.random() - 0.5) * 8, y: (Math.random() - 0.5) * 4 },
          );
          again();
        },
        2500 + Math.random() * 3500,
      );
    };
    again();
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [on]);
  return at;
}

/** Play a one-shot gesture whenever a new one arrives, for long enough to see. */
function useGesture(gesture: string | null | undefined, seq: number): string | null {
  const [playing, setPlaying] = useState<string | null>(null);
  useEffect(() => {
    if (!gesture || seq === 0) return;
    setPlaying(gesture);
    const timer = setTimeout(() => setPlaying(null), 700);
    return () => clearTimeout(timer);
  }, [gesture, seq]);
  return playing;
}

/**
 * Where to look, from what it was told to look at.
 *
 * The `look` port carries a name when it comes from face recognition — "gaze
 * follows whoever is in frame" (§11.3) — and a name is not a place, so the
 * shell decides. Deterministic from the name so a face does not twitch when
 * the same person is recognised twice in a row.
 */
function offsetFor(at: string): { x: number; y: number } {
  let hash = 0;
  for (const ch of at) hash = (hash * 31 + ch.charCodeAt(0)) | 0;
  return { x: ((hash % 13) - 6) * 1.6, y: (((hash >> 4) % 7) - 3) * 1.2 };
}

function clamp(n: number): number {
  return Math.max(0, Math.min(1, n));
}
