import { useEffect, useRef, useState } from 'react';
import { stateOf } from './rigs';
import s from './Face.module.css';

/**
 * The avatar, drawn (SPEC §11.3, §11.4).
 *
 * Three channels, none of them waiting for the others: the expression is what
 * the model or an `express` wire asked for, the mouth is the shape of the audio
 * that is playing, the gaze is wherever `look` pointed. Each has its own clock
 * here, exactly as it does in the engine.
 *
 * Idle is the block's own (§11.4): the blink, the breath and the settle back to
 * neutral happen with nothing running, because an avatar that only moves when
 * the graph does is a picture rather than a presence.
 */
export function Face({
  rig,
  expression = 'neutral',
  intensity = 1,
  mouth,
  gaze,
  idle = true,
  size = 200,
}: {
  rig: string;
  expression?: string;
  intensity?: number;
  /** Loudness over time, 0–255 a bucket, twelve buckets a second. */
  mouth?: number[];
  gaze?: string | null;
  idle?: boolean;
  size?: number;
}) {
  const svg = stateOf(rig, expression);
  const [blinking, setBlinking] = useState(false);
  const openness = useMouth(mouth);

  // The blink. A cadence rather than a metronome: a face that blinks exactly
  // every four seconds reads as a machine pretending, which is worse than a
  // machine.
  useEffect(() => {
    if (!idle) return;
    let alive = true;
    let timer: ReturnType<typeof setTimeout>;
    const again = () => {
      timer = setTimeout(() => {
        if (!alive) return;
        setBlinking(true);
        setTimeout(() => alive && setBlinking(false), 120);
        again();
      }, 2600 + Math.random() * 3200);
    };
    again();
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [idle]);

  if (!svg) {
    return (
      <div className={s.missing} style={{ width: size, height: size }}>
        {rig} has no face to draw here
      </div>
    );
  }

  // Gaze is a small translation rather than a redrawn rig: every rig names its
  // eyes the same thing, so the same nudge works on all four (§11.2's table is
  // about what a rig *can* do, not about how each one does it).
  const drift = gaze ? offsetFor(gaze) : { x: 0, y: 0 };

  return (
    <div
      className={`${s.face} ${idle ? s.alive : ''}`}
      style={{
        width: size,
        height: size,
        // Intensity dims the whole expression rather than redrawing it: a smile
        // at 0.2 is the same smile, held back.
        ['--intensity' as string]: String(0.45 + 0.55 * clamp(intensity)),
        ['--mouth' as string]: String(openness),
        ['--gaze-x' as string]: `${drift.x}px`,
        ['--gaze-y' as string]: `${drift.y}px`,
        ['--blink' as string]: blinking ? '0.06' : '1',
      }}
      data-testid={`face-${rig}`}
      data-expression={expression}
      data-speaking={openness > 0.05 ? 'yes' : 'no'}
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
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
 * Where to look, from what it was told to look at.
 *
 * The `look` port carries a name, not a coordinate — "gaze follows whoever is
 * in frame" (§11.3) — so the shell decides where that person is. Deterministic
 * from the name so a face does not twitch when the same person is recognised
 * twice in a row.
 */
function offsetFor(at: string): { x: number; y: number } {
  let hash = 0;
  for (const ch of at) hash = (hash * 31 + ch.charCodeAt(0)) | 0;
  return { x: ((hash % 13) - 6) * 1.6, y: (((hash >> 4) % 7) - 3) * 1.2 };
}

function clamp(n: number): number {
  return Math.max(0, Math.min(1, n));
}
