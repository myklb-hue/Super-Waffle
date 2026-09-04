/**
 * Talking to the engine.
 *
 * The engine is a separate process, so the shell reaches it one of two ways:
 *
 *  - through the Tauri host, which owns the socket and forwards a request;
 *  - over HTTP, which is what `npm run dev` uses in a plain browser and what
 *    the screenshot tests drive.
 *
 * A request and its reply are one round trip. Everything a run has to say
 * comes the other way, unprompted, so there is a second channel for it:
 * `subscribeEvents` below.
 *
 * Everything above this file works in terms of `call`, so neither path leaks
 * into the components.
 */

import type {
  Decision,
  Graph,
  OpenGraph,
  Reply,
  Request,
  RunEvent,
  RunStarted,
  Saved,
} from '@cyberloom/graph-core';

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export const isTauri = () => typeof window !== 'undefined' && !!window.__TAURI_INTERNALS__;

export class EngineUnreachable extends Error {
  constructor(cause: unknown) {
    super(
      cause instanceof Error ? cause.message : 'the engine did not answer',
    );
    this.name = 'EngineUnreachable';
  }
}

async function callTauri(request: Request): Promise<Reply> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<Reply>('rpc', { request });
}

async function callHttp(request: Request): Promise<Reply> {
  const response = await fetch('/rpc', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(request),
  });
  if (!response.ok) throw new Error(`the engine answered ${response.status}`);
  return (await response.json()) as Reply;
}

/** One request, one reply. A transport failure throws; an engine-level failure
 *  comes back as `Reply::Error`, because that is something to show, not to
 *  crash on (SPEC §12.1). */
export async function call(request: Request): Promise<Reply> {
  try {
    return isTauri() ? await callTauri(request) : await callHttp(request);
  } catch (cause) {
    throw new EngineUnreachable(cause);
  }
}

export async function openGraph(path: string): Promise<OpenGraph> {
  const reply = await call({ method: 'graph.open', params: { path } });
  if (reply.result === 'error') throw new Error(reply.data.message);
  if (reply.result !== 'graph') throw new Error(`unexpected reply: ${reply.result}`);
  return reply.data;
}

/** Write the graph back. The reply carries the canonical form the engine
 *  actually wrote, which the shell adopts (see `autosave.ts`). */
export async function saveGraph(path: string, graph: Graph): Promise<Saved> {
  const reply = await call({ method: 'graph.save', params: { path, graph } });
  if (reply.result === 'error') throw new Error(reply.data.message);
  if (reply.result !== 'saved') throw new Error(`unexpected reply: ${reply.result}`);
  return reply.data;
}

export async function engineStatus() {
  const reply = await call({ method: 'engine.status' });
  if (reply.result !== 'engineStatus') throw new Error('the engine did not report its status');
  return reply.data;
}

export async function listWorkspace() {
  const reply = await call({ method: 'workspace.list' });
  if (reply.result !== 'workspace') throw new Error('the engine did not list the workspace');
  return reply.data;
}

// ------------------------------------------------------------------- events

/**
 * What a run says while it is in flight.
 *
 * The two transports differ only in how a line reaches the window: the Tauri
 * host emits it as a window event, the dev server as an EventSource message.
 * Everything above this sees one stream of `RunEvent`.
 */
export type EventHandler = (event: RunEvent) => void;

export function subscribeEvents(handler: EventHandler): () => void {
  return isTauri() ? subscribeTauri(handler) : subscribeSse(handler);
}

function subscribeTauri(handler: EventHandler): () => void {
  let stop: (() => void) | null = null;
  let cancelled = false;
  void (async () => {
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<RunEvent>('loomd:event', (message) =>
      handler(message.payload),
    );
    // The caller may have unsubscribed while the import was in flight.
    if (cancelled) unlisten();
    else stop = unlisten;
  })();
  return () => {
    cancelled = true;
    stop?.();
  };
}

function subscribeSse(handler: EventHandler): () => void {
  const source = new EventSource('/events');
  source.onmessage = (message) => {
    try {
      handler(JSON.parse(message.data as string) as RunEvent);
    } catch {
      // A malformed line is the engine's to report. Dropping it beats
      // tearing the stream down over one bad object.
    }
  };
  return () => source.close();
}

/** Run a graph. The reply is immediate; the run itself arrives as events. */
export async function startRun(path: string, graph: Graph): Promise<RunStarted> {
  const reply = await call({ method: 'run.start', params: { path, graph } });
  if (reply.result === 'error') throw new Error(reply.data.message);
  if (reply.result !== 'running') throw new Error(`unexpected reply: ${reply.result}`);
  return reply.data;
}

export async function stopRun(run?: string): Promise<number> {
  const reply = await call({ method: 'run.stop', params: { run: run ?? null } });
  return reply.result === 'acknowledged' ? reply.data.count : 0;
}

/** Answer a warning the run is parked on (SPEC §12.1). */
export async function answerWarning(warning: string, decision: Decision): Promise<boolean> {
  const reply = await call({ method: 'run.continue', params: { warning, decision } });
  return reply.result === 'acknowledged' && reply.data.ok;
}
