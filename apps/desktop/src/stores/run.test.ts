import { beforeEach, describe, expect, it } from 'vitest';
import { useRun } from './run';
import type { RunEvent } from '@cyberloom/graph-core';

/** One face event, with the fields a test does not care about filled in. */
function face(over: Partial<Extract<RunEvent, { event: 'face' }>['data']>): RunEvent {
  return {
    event: 'face',
    data: {
      run: 'r1',
      block: 'face',
      rig: 'line',
      expression: 'neutral',
      intensity: 1,
      mouth: [],
      gaze: null,
      gazeAt: null,
      gesture: null,
      asleep: false,
      blinkMs: 4000,
      breathePerMin: 13,
      colour: '#56c7d6',
      ...over,
    },
  };
}

describe('the face in the run store (SPEC §11)', () => {
  beforeEach(() => {
    useRun.getState().apply({
      event: 'run.started',
      data: { run: 'r1', graph: 'g', order: [] },
    });
  });

  it('keeps what the engine said, including where it is looking', () => {
    useRun.getState().apply(face({ expression: 'smile', gaze: 'Mykl', gazeAt: [0.25, 0.75] }));
    const worn = useRun.getState().faces.face!;
    expect(worn.expression).toBe('smile');
    expect(worn.gaze).toBe('Mykl');
    expect(worn.gazeAt).toEqual([0.25, 0.75]);
    expect(worn.blinkMs).toBe(4000);
    expect(worn.colour).toBe('#56c7d6');
  });

  it('counts gestures, so the same one twice plays twice and a plain event does not replay it', () => {
    const apply = useRun.getState().apply;
    apply(face({ gesture: 'nod' }));
    expect(useRun.getState().faces.face!.gestureSeq).toBe(1);
    apply(face({ gesture: 'nod' }));
    expect(useRun.getState().faces.face!.gestureSeq).toBe(2);
    // A mouth update carries no gesture and must not nod again.
    apply(face({ mouth: [10, 200, 40] }));
    const after = useRun.getState().faces.face!;
    expect(after.gestureSeq).toBe(2);
    expect(after.gesture).toBe('nod');
    expect(after.mouth).toEqual([10, 200, 40]);
  });

  it('knows when a face is asleep, and per block', () => {
    const apply = useRun.getState().apply;
    apply(face({ block: 'face', asleep: true, expression: 'sleepy' }));
    apply(face({ block: 'lamp', rig: 'line', colour: '#6fc98a', expression: 'smile' }));
    expect(useRun.getState().faces.face!.asleep).toBe(true);
    expect(useRun.getState().faces.lamp!.asleep).toBe(false);
    expect(useRun.getState().faces.lamp!.colour).toBe('#6fc98a');
  });
});
