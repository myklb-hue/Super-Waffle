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
  let nextId = 1;

  function start(): Engine {
    if (child) return child;
    const workspace = process.env.CYBERLOOM_WORKSPACE ?? `${root}fixtures`;
    const binary = process.env.LOOMD ?? `${root}target/debug/loomd`;
    const started = spawn(binary, ['--workspace', workspace], {
      stdio: ['pipe', 'pipe', 'inherit'],
    });
    createInterface({ input: started.stdout }).on('line', (line) => {
      try {
        const reply = JSON.parse(line) as { id: number };
        pending.get(reply.id)?.(reply);
        pending.delete(reply.id);
      } catch {
        // A line that will not parse is the engine's to report, not something
        // to bring the dev server down for.
      }
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
      server.httpServer?.on('close', () => child?.kill());
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
