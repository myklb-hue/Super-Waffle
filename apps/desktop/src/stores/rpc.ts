/**
 * Talking to the engine.
 *
 * The engine is a separate process, so the shell reaches it one of two ways:
 *
 *  - through the Tauri host, which owns the socket and forwards a request;
 *  - over HTTP, which is what `npm run dev` uses in a plain browser and what
 *    the screenshot tests drive.
 *
 * Everything above this file works in terms of `call`, so neither path leaks
 * into the components.
 */

import type { OpenGraph, Reply, Request } from '@cyberloom/graph-core';

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
