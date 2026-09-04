import { useCallback, useRef } from 'react';
import s from './ui.module.css';

export interface GripProps {
  disabled?: boolean;
  /** Called with the movement since the last event, not since the drag began. */
  onResize: (dx: number, dy: number) => void;
  onResizeEnd?: () => void;
}

/** The corner handle on any block with a picture. Uses pointer capture so the
 *  drag survives the pointer leaving the block (SPEC 3.4). */
export function Grip({ disabled = false, onResize, onResizeEnd }: GripProps) {
  const last = useRef<{ x: number; y: number } | null>(null);

  const onPointerDown = useCallback((e: React.PointerEvent<HTMLButtonElement>) => {
    e.preventDefault();
    e.currentTarget.setPointerCapture(e.pointerId);
    last.current = { x: e.clientX, y: e.clientY };
  }, []);

  const onPointerMove = useCallback(
    (e: React.PointerEvent<HTMLButtonElement>) => {
      if (!last.current) return;
      onResize(e.clientX - last.current.x, e.clientY - last.current.y);
      last.current = { x: e.clientX, y: e.clientY };
    },
    [onResize],
  );

  const onPointerUp = useCallback(
    (e: React.PointerEvent<HTMLButtonElement>) => {
      if (!last.current) return;
      e.currentTarget.releasePointerCapture(e.pointerId);
      last.current = null;
      onResizeEnd?.();
    },
    [onResizeEnd],
  );

  return (
    <button
      type="button"
      className={s.grip}
      disabled={disabled}
      aria-label="Resize"
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
    >
      <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
        <path
          d="M11 5L5 11M11 9l-2 2"
          stroke="currentColor"
          strokeWidth="1.4"
          strokeLinecap="round"
          fill="none"
        />
      </svg>
    </button>
  );
}
