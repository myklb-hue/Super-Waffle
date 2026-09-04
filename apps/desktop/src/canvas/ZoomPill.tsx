import { useReactFlow, useStore } from '@xyflow/react';
import { Icon } from '@cyberloom/ui';
import s from './ZoomPill.module.css';

/**
 * The floating zoom control. It reads the live viewport rather than keeping a
 * number of its own, so scrolling to zoom and pressing the button can never
 * disagree.
 */
export function ZoomPill() {
  const zoom = useStore((state) => state.transform[2]);
  const { zoomIn, zoomOut, fitView } = useReactFlow();

  return (
    <div className={s.pill}>
      <button type="button" className={s.button} aria-label="Zoom out" onClick={() => zoomOut()}>
        <Icon name="minus" size={12} strokeWidth={2} />
      </button>
      <span className={s.value}>{Math.round(zoom * 100)}%</span>
      <button type="button" className={s.button} aria-label="Zoom in" onClick={() => zoomIn()}>
        <Icon name="plus" size={12} strokeWidth={2} />
      </button>
      <button
        type="button"
        className={s.button}
        aria-label="Fit to view"
        onClick={() => fitView({ padding: 0.12, maxZoom: 1 })}
      >
        <Icon name="fit" size={12} strokeWidth={1.7} />
      </button>
    </div>
  );
}
