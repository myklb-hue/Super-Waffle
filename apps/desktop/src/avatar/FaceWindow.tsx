import { useEffect, useState } from 'react';
import { Face } from './Face';
import { refreshRigs } from './rigs';
import { listenToRuns, useRun } from '../stores/run';
import s from './FaceWindow.module.css';

/**
 * A window that is only a face (SPEC §11.5).
 *
 * Opened with `?face=<block>`. It listens to the same run events the main
 * window does and draws whatever that block's face is doing, as large as the
 * window allows. Nothing else: no canvas, no inspector, no controls. Until the
 * first face event arrives it wears the rig the URL named, resting.
 */
export function FaceWindow({ block, rig }: { block: string; rig: string }) {
  const face = useRun((r) => r.faces[block]);
  const size = useWindowSquare();

  useEffect(() => {
    void refreshRigs();
    return listenToRuns();
  }, []);

  return (
    <div className={s.stage} data-testid="face-window">
      <Face
        rig={face?.rig ?? rig}
        expression={face?.expression}
        intensity={face?.intensity}
        mouth={face?.mouth}
        gaze={face?.gaze}
        gazeAt={face?.gazeAt}
        gesture={face?.gesture}
        gestureSeq={face?.gestureSeq}
        asleep={face?.asleep}
        blinkMs={face?.blinkMs}
        breathePerMin={face?.breathePerMin}
        size={size}
      />
    </div>
  );
}

/** The largest square that fits the window, kept current as it is resized. */
function useWindowSquare(): number {
  const measure = () => Math.max(96, Math.min(window.innerWidth, window.innerHeight) - 24);
  const [size, setSize] = useState(measure);
  useEffect(() => {
    const onResize = () => setSize(measure());
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);
  return size;
}
