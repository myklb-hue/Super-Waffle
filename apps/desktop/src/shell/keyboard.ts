/**
 * The keyboard map.
 *
 * It lives here rather than on the canvas because these are window-level
 * gestures: undo does not care what has focus, and delete has to work whether
 * the pointer is over the canvas or the inspector. Anything typed into a field
 * is left alone, which is what the `isTyping` guard is for.
 */

import { useEffect } from 'react';
import { useDocument } from '../stores/document';

function isTyping(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el) return false;
  return (
    el.tagName === 'INPUT' ||
    el.tagName === 'TEXTAREA' ||
    el.isContentEditable === true
  );
}

export function useKeyboard() {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const store = useDocument.getState();
      const temporal = useDocument.temporal.getState();
      const mod = event.metaKey || event.ctrlKey;

      if (mod && event.key.toLowerCase() === 'z') {
        event.preventDefault();
        if (event.shiftKey) temporal.redo();
        else temporal.undo();
        // Time travel restores the graph but knows nothing about `dirty`, so
        // without this an undone edit is never written back to the file.
        store.touch();
        return;
      }
      if (mod && event.key.toLowerCase() === 'y') {
        event.preventDefault();
        temporal.redo();
        store.touch();
        return;
      }

      if (isTyping(event.target)) return;

      if (event.key === 'Delete' || event.key === 'Backspace') {
        if (store.selection.kind === 'none') return;
        event.preventDefault();
        store.deleteSelection();
        return;
      }

      if (event.key === 'Escape') {
        store.select({ kind: 'none' });
        return;
      }

      // A view toggle from the keyboard, for the selected blocks.
      if (store.selection.kind === 'block') {
        const view = { '1': 'compact', '2': 'summary', '3': 'stage' } as const;
        const chosen = view[event.key as keyof typeof view];
        if (chosen) {
          event.preventDefault();
          for (const id of store.selection.ids) {
            const block = store.graph.blocks.find((b) => b.id === id);
            if (!block) continue;
            // A block with no third view has nothing to switch to.
            if (chosen === 'stage' && block.kind !== 'custom') {
              store.setBlockView(id, 'stage');
            } else if (chosen === 'stage') {
              store.setBlockView(id, 'code');
            } else {
              store.setBlockView(id, chosen);
            }
          }
        }
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
}
