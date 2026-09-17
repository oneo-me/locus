import { build } from 'vite';
import react from '@vitejs/plugin-react';
import tailwind from '@tailwindcss/vite';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { openLibrary } from './library.mjs';
import { hostDirectory, locusPlugin, pageHtml } from './plugin.mjs';

export async function exportCanvas(root, note, directory, { resources = [], dark = false, install = true } = {}) {
  const library = await openLibrary(root);
  await library.component(note);
  const addresses = new Set(resources);
  const mappings = {};
  const index = path.join(hostDirectory, 'canvas.html');
  await build({
    configFile: false, root: hostDirectory, publicDir: false, base: './',
    define: { 'process.env.NODE_ENV': '"production"' },
    plugins: [locusPlugin(library, { install, resourceModule: 'export default globalThis.__LOCUS_RESOURCE_MAP__;' }), react(), tailwind(), {
      name: 'locus-export',
      enforce: 'pre',
      resolveId(id) { if (id === index) return id; },
      load(id) {
        if (id === index) return pageHtml(note, dark);
      },
      transform(source, id) {
        if (!id.includes('node_modules')) {
          for (const match of source.matchAll(/["'`](locus:\/\/[^"'`\s$]+\/(?:assets\/[^"'`\s]+|canvas\/[^"'`\s]+|index\.md(?:\?part=(?:yaml|body))?))["'`]/g)) {
            addresses.add(match[1]);
          }
        }
        if (id === path.join(hostDirectory, 'host.tsx')) {
          return source.replace('import(/* @vite-ignore */ `/__locus/entry.tsx?note=${token}&import`)', `import(${JSON.stringify(`locus:${note}`)})`);
        }
      },
      async buildEnd(error) {
        if (error) return;
        for (const address of addresses) {
          const { file, bytes } = await library.resource(address);
          const name = createHash('sha256').update(address).digest('hex').slice(0, 16) + path.extname(file);
          const reference = this.emitFile({ type: 'asset', fileName: `resources/${name}`, source: bytes });
          mappings[address] = `./${this.getFileName(reference)}`;
        }
      },
      renderChunk(code) {
        return { code: code.replaceAll('globalThis.__LOCUS_RESOURCE_MAP__', JSON.stringify(mappings)), map: null };
      },
    }],
    resolve: { dedupe: ['react', 'react-dom'], preserveSymlinks: true },
    build: { outDir: directory, emptyOutDir: false, rollupOptions: { input: index } },
  });
}
