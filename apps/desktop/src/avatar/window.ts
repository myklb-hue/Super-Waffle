/**
 * The face in a window of its own (SPEC §11.5).
 *
 * "A window (optionally always on top), a specific screen": the Avatar's
 * `output` setting says so, and this opens one. In the Tauri host it is a
 * second webview on the same event stream, which the host positions on the
 * screen asked for and keeps on top if asked; in a plain browser it is a
 * popup on the same origin. Both load the shell with `?face=<block>`, and the
 * shell draws only that face.
 *
 * Idempotent per block: the host focuses a window it already has, and the
 * browser reuses a named popup.
 */
import { isTauri } from '../stores/rpc';

export interface WindowChoice {
  alwaysOnTop: boolean;
  /** Which monitor, counting from zero, or none for wherever the host puts it. */
  screen: number | null;
}

const opened = new Map<string, Window | null>();

export async function openFaceWindow(block: string, rig: string, choice: WindowChoice): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('face_window', {
      block,
      rig,
      alwaysOnTop: choice.alwaysOnTop,
      screen: choice.screen,
    });
    return;
  }
  const url = new URL(window.location.href);
  url.search = `?face=${encodeURIComponent(block)}&rig=${encodeURIComponent(rig)}`;
  const already = opened.get(block);
  if (already && !already.closed) {
    already.focus();
    return;
  }
  opened.set(block, window.open(url.toString(), `cyberloom-face-${block}`, 'width=360,height=360'));
}
