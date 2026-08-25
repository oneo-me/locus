import { serve } from '@hono/node-server';
import { existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createApp } from './app';
import { closeLibrary } from './features/library/instance.server';
import { logError, logInfo } from './logging';

const clientDirectory = fileURLToPath(new URL('./client', import.meta.url));
const staticRoot = existsSync(join(clientDirectory, 'index.html'))
  ? clientDirectory
  : undefined;

export function startServer(port = 3001) {
  return serve({
    fetch: createApp(staticRoot).fetch,
    hostname: '127.0.0.1',
    port,
  });
}

function parsePort(value: string | undefined): number {
  if (value === undefined) return 3001;
  if (!/^\d+$/.test(value)) {
    throw new Error('端口必须是 1 到 65535 之间的整数。');
  }
  const port = Number(value);
  if (port < 1 || port > 65535) {
    throw new Error('端口必须是 1 到 65535 之间的整数。');
  }
  return port;
}

async function close(server: ReturnType<typeof startServer>) {
  await new Promise<void>((resolveClose, reject) => {
    server.close((error) => (error ? reject(error) : resolveClose()));
  });
  await closeLibrary();
}

const entryUrl = process.argv[1]
  ? pathToFileURL(resolve(process.argv[1])).href
  : undefined;

function run() {
  let port: number;
  try {
    port = parsePort(process.argv[2]);
  } catch (error) {
    logError('server.start_failed', error);
    process.exitCode = 1;
    return;
  }

  const server = startServer(port);
  server.once('listening', () =>
    logInfo('server.started', { address: `http://127.0.0.1:${port}` }),
  );
  server.on('error', (error) => {
    logError('server.failed', error);
    process.exitCode = 1;
  });

  let closing = false;
  const closeOnce = (signal: NodeJS.Signals) => {
    if (closing) return;
    closing = true;
    logInfo('server.stopping', { signal });
    void close(server)
      .then(() => logInfo('server.stopped'))
      .catch((error) => {
        logError('server.stop_failed', error);
        process.exitCode = 1;
      });
  };

  process.once('SIGINT', closeOnce);
  process.once('SIGTERM', closeOnce);
}

if (import.meta.url === entryUrl) run();
