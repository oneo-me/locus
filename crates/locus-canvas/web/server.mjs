import { createServer } from 'vite';
import react from '@vitejs/plugin-react';
import tailwind from '@tailwindcss/vite';
import { openLibrary } from './library.mjs';
import { hostDirectory, locusPlugin, pageHtml } from './plugin.mjs';
import { pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';
import path from 'node:path';

export async function startServer(root, { port = 0, install = true } = {}) {
  const library = await openLibrary(root);
  const dependencies = new Map();
  const server = await createServer({
    configFile: false,
    root: hostDirectory,
    publicDir: false,
    cacheDir: path.join(hostDirectory, 'node_modules/.vite', createHash('sha256').update(library.root).digest('hex').slice(0, 16)),
    appType: 'custom',
    clearScreen: false,
    plugins: [{
      name: 'locus-local-access',
      configureServer(instance) {
        instance.middlewares.use((request, response, next) => {
          const host = request.headers.host;
          const origin = request.headers.origin;
          if (!/^127\.0\.0\.1:\d+$/.test(host ?? '') || origin && origin !== `http://${host}`) {
            response.writeHead(403).end('Canvas is available only to its local host');
            return;
          }
          next();
        });
      },
    }, locusPlugin(library, { install, onResource(referer, address) {
      if (!referer) return;
      const owner = new URL(referer).pathname;
      if (!dependencies.has(owner)) dependencies.set(owner, new Set());
      dependencies.get(owner).add(address);
    } }), react(), tailwind(), {
      name: 'locus-pages',
      configureServer(instance) {
        return () => instance.middlewares.use(async (request, response, next) => {
          const url = new URL(request.url, 'http://localhost');
          if (url.pathname === '/__locus/dependencies') {
            response.writeHead(200, { 'Content-Type': 'application/json' });
            response.end(JSON.stringify([...(dependencies.get(url.searchParams.get('page')) ?? [])]));
            return;
          }
          if (!url.pathname.startsWith('/notes/')) return next();
          try {
            const note = `locus://${url.pathname.slice(7).replace(/\/$/, '')}`;
            await library.component(note);
            dependencies.delete(url.pathname);
            const html = await instance.transformIndexHtml(url.pathname, pageHtml(note, false, url.searchParams.get('lang')));
            response.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store' });
            response.end(html);
          } catch (error) {
            response.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' }).end(error.message);
          }
        });
      },
    }],
    resolve: { dedupe: ['react', 'react-dom'], preserveSymlinks: true },
    optimizeDeps: { include: ['react', 'react-dom/client', 'react/jsx-runtime', 'react/jsx-dev-runtime'] },
    server: {
      host: '127.0.0.1', port, strictPort: true, cors: false,
      fs: { strict: true, allow: [hostDirectory] },
      headers: {
        'X-Content-Type-Options': 'nosniff', 'Referrer-Policy': 'same-origin',
        'Content-Security-Policy': "default-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' https: data: blob:; font-src 'self' data:; media-src 'self' data: blob:; connect-src 'self' ws://127.0.0.1:* https:; worker-src 'self' blob:; frame-src 'none'; object-src 'none'; base-uri 'self'; form-action 'none'",
      },
    },
  });
  try {
    await server.listen();
  } catch (error) {
    await server.close();
    throw error;
  }
  return server;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const server = await startServer(process.argv[2]);
  const address = server.httpServer.address();
  process.stdout.write(`LOCUS_READY http://127.0.0.1:${address.port}\n`);
  const shutdown = async () => { await server.close(); process.exit(0); };
  process.stdin.resume();
  process.stdin.on('end', shutdown);
  process.on('SIGTERM', shutdown);
  process.on('SIGINT', shutdown);
}
