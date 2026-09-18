import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdir, realpath, symlink, rm } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import { Packages } from './packages.mjs';

export const hostDirectory = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const mime = { '.svg': 'image/svg+xml', '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg',
  '.webp': 'image/webp', '.gif': 'image/gif', '.css': 'text/css', '.json': 'application/json', '.woff2': 'font/woff2',
  '.woff': 'font/woff', '.mp4': 'video/mp4', '.pdf': 'application/pdf', '.js': 'text/javascript', '.html': 'text/html' };

export function locusPlugin(library, { install = true } = {}) {
  const packages = new Packages();
  let server;
  const resources = new Map();
  const styles = new Map();
  const entries = new Map();
  const allowed = new Set([hostDirectory]);
  const mountRoot = path.join(hostDirectory, '.locus-packages', randomUUID());
  const mounts = new Map();
  const permitted = file => [...allowed].some(directory => file === directory || file.startsWith(directory + path.sep));
  return {
    name: 'locus-canvas',
    enforce: 'pre',
    async resolveId(id) {
      if (id === '@locus/canvas') return path.join(hostDirectory, 'sdk.tsx');
      if (/^(react|react-dom)(\/|$)/.test(id)) return require.resolve(id);
      if (id === 'tailwindcss' || id.startsWith('tailwindcss/')) return require.resolve(id === 'tailwindcss' ? 'tailwindcss/index.css' : id);
      if (id.startsWith('locus-style:')) return id;
      const token = id.startsWith('/__locus/entry.tsx?') ? new URL(id, 'http://localhost').searchParams.get('note') : null;
      const route = token ? Buffer.from(token, 'base64url').toString('utf8') : null;
      if (!route && !id.startsWith('locus:')) return;
      const [note, entry] = (route ?? id.slice(6)).split('#');
      const address = note.startsWith('locus://') ? note : `locus://${note}`;
      const component = await library.component(address, entry ? `./${entry}` : '.');
      allowed.add(component.directory);
      await mkdir(mountRoot, { recursive: true });
      const mount = path.join(mountRoot, createHash('sha256').update(component.directory).digest('hex').slice(0, 16));
      // Vite module IDs treat # and ? as URL syntax; keep note names out of them.
      try { await symlink(component.directory, mount, 'junction'); }
      catch (error) { if (error.code !== 'EEXIST' || await realpath(mount) !== component.directory) throw error; }
      const source = path.join(mount, path.relative(component.directory, component.source));
      mounts.set(component.directory, mount);
      const style = `locus-style:${Buffer.from(component.directory).toString('hex')}.css`;
      styles.set(style, component.directory);
      entries.set(source, style);
      if (install) await packages.ensure(component.directory);
      if (server) {
        if (!server.config.server.fs.allow.includes(component.directory)) server.config.server.fs.allow.push(component.directory);
        server.watcher.add([component.directory, mount]);
      }
      return source;
    },
    async load(id) {
      const directory = styles.get(id);
      if (directory) return `@reference ${JSON.stringify(require.resolve('tailwindcss/theme.css'))};
@import ${JSON.stringify(require.resolve('tailwindcss/utilities.css'))} source(none);
@custom-variant dark (&:where(.dark, .dark *));
@source ${JSON.stringify(directory.replaceAll('\\', '/'))};`;
      const file = id.split('?')[0];
      const actual = path.isAbsolute(file) ? await realpath(file).catch(() => null) : null;
      if (actual && !permitted(actual)) {
        throw new Error('Canvas imports must stay within declared note packages and the host runtime');
      }
    },
    transform(code, id) {
      const style = entries.get(id.split('?')[0]);
      if (style) return { code: `import ${JSON.stringify(style)};\n${code}`, map: null };
    },
    handleHotUpdate(update) {
      const modules = new Set(update.modules);
      for (const [directory, mount] of mounts) {
        if (!update.file.startsWith(directory + path.sep)) continue;
        const file = path.join(mount, path.relative(directory, update.file));
        for (const module of update.server.moduleGraph.getModulesByFile(file) ?? []) modules.add(module);
      }
      return [...modules];
    },
    async closeBundle() {
      await rm(mountRoot, { recursive: true, force: true });
    },
    configureServer(instance) {
      server = instance;
      server.watcher.on('all', (event, file) => {
        if (!['change', 'add', 'unlink'].includes(event)) return;
        for (const uri of resources.get(file) ?? []) {
          server.ws.send({ type: 'custom', event: 'locus:resource-changed', data: { uri } });
        }
      });
      server.watcher.on('change', async file => {
        if (path.basename(file) !== 'package.json' || ![...styles.values()].includes(path.dirname(file))) return;
        try {
          if (install) await packages.ensure(path.dirname(file));
          await server.restart();
        } catch (error) {
          server.ws.send({ type: 'error', err: { message: error.message, stack: error.stack } });
        }
      });
      server.middlewares.use(async (request, response, next) => {
        const url = new URL(request.url, 'http://localhost');
        let pathname;
        try { pathname = decodeURIComponent(url.pathname); }
        catch { response.writeHead(400).end('Invalid resource URL'); return; }
        if (url.pathname.startsWith('/.locus-packages/')) {
          const file = path.join(hostDirectory, pathname);
          const actual = await realpath(file).catch(() => null);
          if (actual && !permitted(actual)) {
            response.writeHead(403).end('Resource is outside declared canvas packages');
            return;
          }
        }
        if (url.pathname.startsWith('/@fs/')) {
          const file = pathname.slice(4);
          if (!permitted(await realpath(file).catch(() => file))) {
            response.writeHead(403).end('Resource is outside declared canvas packages');
            return;
          }
        }
        if (url.pathname !== '/__locus/resource') return next();
        if (!['GET', 'HEAD'].includes(request.method)) {
          response.writeHead(405).end();
          return;
        }
        try {
          const address = url.searchParams.get('uri') ?? '';
          const resource = await library.resource(address);
          if (!resources.has(resource.file)) resources.set(resource.file, new Set());
          resources.get(resource.file).add(address.split('?')[0]);
          server.watcher.add(resource.file);
          response.writeHead(200, { 'Content-Type': mime[path.extname(resource.file)] ?? 'text/plain; charset=utf-8',
            'Cache-Control': 'no-store', 'X-Content-Type-Options': 'nosniff' });
          response.end(request.method === 'HEAD' ? undefined : resource.bytes);
        } catch (error) {
          response.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' }).end(error.message);
        }
      });
    },
  };
}

export function pageHtml(note, dark = false, language = 'en') {
  const escaped = note.replaceAll('&', '&amp;').replaceAll('"', '&quot;').replaceAll('<', '&lt;');
  return `<!doctype html><html lang="${language === 'zh-CN' ? 'zh-CN' : 'en'}" class="${dark ? 'dark' : 'light'}" data-note="${escaped}">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"></head>
<body><div id="root"></div><script type="module" src="/host.tsx"></script></body></html>`;
}
