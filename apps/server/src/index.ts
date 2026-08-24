import { serve } from '@hono/node-server';
import { createApp } from './app';
import { closeLibrary } from './features/library/instance.server';

const hostname = process.env.LOCUS_SERVER_HOST?.trim() || '127.0.0.1';
const port = Number.parseInt(
  process.env.LOCUS_SERVER_PORT?.trim() || '3001',
  10,
);
const server = serve({ fetch: createApp().fetch, hostname, port });

console.log(`Locus server listening on http://${hostname}:${port}`);

let closing = false;
async function close() {
  if (closing) return;
  closing = true;
  server.close();
  await closeLibrary();
}

process.once('SIGINT', () => void close());
process.once('SIGTERM', () => void close());
