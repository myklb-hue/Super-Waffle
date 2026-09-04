import { spawn, type ChildProcessByStdio } from 'node:child_process';
import type { Readable, Writable } from 'node:stream';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import react from '@vitejs/plugin-react';
import { defineConfig, type Plugin } from 'vite';

const root = fileURLToPath(new URL('../..', import.meta.url));

/**
 * Runs the engine beside the dev server and forwards `/rpc` to it.
 *
 * This is the same arrangement the Tauri host uses — engine as a child process,
 * one JSON object per line over stdio — so developing in a browser exercises
 * the real protocol rather than a mock. Dev and test only; a packaged build has
 * the host instead.
 */
function engine(): Plugin {
  type Engine = ChildProcessByStdio<Writable, Readable, null>;
  let child: Engine | null = null;
  const pending = new Map<number, (reply: unknown) => void>();
  /** Everyone listening on /events. A run's events go to all of them. */
  const listeners = new Set<import('node:http').ServerResponse>();
  let nextId = 1;
  /**
   * The folder the current engine is serving.
   *
   * One workspace per engine is the engine's own rule, so switching workspaces
   * is starting a different engine — which is exactly what the Tauri host does
   * too. Here the shell asks by sending `?workspace=`, and the child is
   * replaced.
   */
  let serving: string | null = null;

  function start(): Engine {
    if (child) return child;
    const workspace = serving ?? process.env.CYBERLOOM_WORKSPACE ?? `${root}fixtures`;
    serving = workspace;
    const binary = process.env.LOOMD ?? `${root}target/debug/loomd`;
    const started = spawn(binary, ['--workspace', workspace], {
      stdio: ['pipe', 'pipe', 'inherit'],
    });
    createInterface({ input: started.stdout }).on('line', (line) => {
      let message: { id?: number };
      try {
        message = JSON.parse(line) as { id?: number };
      } catch {
        // A line that will not parse is the engine's to report, not something
        // to bring the dev server down for.
        return;
      }
      // A reply carries the id of the request it answers; an event carries
      // none and goes to whoever is listening. That one distinction is the
      // whole of the demultiplexing.
      if (message.id === undefined) {
        for (const listener of listeners) listener.write(`data: ${line}\n\n`);
        return;
      }
      pending.get(message.id)?.(message);
      pending.delete(message.id);
    });
    started.on('exit', () => {
      child = null;
      for (const resolve of pending.values()) {
        resolve({ result: 'error', data: { code: 'engine', message: 'the engine stopped' } });
      }
      pending.clear();
    });
    child = started;
    return started;
  }

  return {
    name: 'cyberloom-engine',
    configureServer(server) {
      // Which folder to serve. A different one replaces the child: one
      // workspace per engine (see `Engine` in the crate), so a second workspace
      // is a second engine rather than a mode inside this one.
      server.middlewares.use('/workspace', (req, res) => {
        const wanted = new URL(req.url ?? '/', 'http://x').searchParams.get('path');
        if (wanted && wanted !== serving) {
          serving = wanted;
          child?.kill();
          child = null;
        }
        res.setHeader('content-type', 'application/json');
        res.end(JSON.stringify({ workspace: serving ?? `${root}fixtures` }));
      });

      server.middlewares.use('/rpc', (req, res) => {
        const chunks: Buffer[] = [];
        req.on('data', (c: Buffer) => chunks.push(c));
        req.on('end', () => {
          const id = nextId++;
          const request = JSON.parse(Buffer.concat(chunks).toString() || '{}') as object;
          const timer = setTimeout(() => {
            pending.delete(id);
            res.statusCode = 504;
            res.end(
              '{"result":"error","data":{"code":"timeout","message":"the engine did not answer"}}',
            );
          }, 5000);
          pending.set(id, (reply) => {
            clearTimeout(timer);
            res.setHeader('content-type', 'application/json');
            res.end(JSON.stringify(reply));
          });
          try {
            start().stdin.write(`${JSON.stringify({ id, ...request })}\n`);
          } catch (e) {
            clearTimeout(timer);
            pending.delete(id);
            res.statusCode = 502;
            res.end(
              JSON.stringify({ result: 'error', data: { code: 'engine', message: String(e) } }),
            );
          }
        });
      });
      // Events, as an EventSource stream. The Tauri build has the host's own
      // event channel instead; this is the browser's equivalent, so developing
      // in a tab watches a real run rather than a mock of one.
      server.middlewares.use('/events', (req, res) => {
        res.writeHead(200, {
          'content-type': 'text/event-stream',
          'cache-control': 'no-cache',
          connection: 'keep-alive',
        });
        // Vite sits behind no proxy here, but a comment line makes the stream
        // open immediately rather than when the first event arrives.
        res.write(': open\n\n');
        listeners.add(res);
        start();
        req.on('close', () => listeners.delete(res));
      });

      server.httpServer?.on('close', () => {
        for (const listener of listeners) listener.end();
        child?.kill();
      });
    },
  };
}

export default defineConfig({
  plugins: [react(), engine()],
  // Tauri serves the built files from disk, so every asset reference has to be
  // relative rather than rooted at /.
  base: './',
  build: { outDir: 'dist', emptyOutDir: true, target: 'safari16' },
  server: { port: 5173, strictPort: true },
  clearScreen: false,
});
