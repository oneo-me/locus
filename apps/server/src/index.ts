import { serve } from '@hono/node-server';
import { createApp } from './app';
import { closeLibrary } from './features/library/instance.server';

const server = serve({
  fetch: createApp().fetch,
  hostname: '127.0.0.1',
  port: 3001,
});

console.log('Locus server listening on http://127.0.0.1:3001');

let closing = false;
async function close() {
  if (closing) return;
  closing = true;
  server.close();
  await closeLibrary();
}

process.once('SIGINT', () => void close());
process.once('SIGTERM', () => void close());
