import { reactRouter } from '@react-router/dev/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

const serverRuntimeDependencies = ['@react-router/node', 'sharp'] as const;
const isE2E = process.env.ASTERWEFT_E2E === '1';
const webHost = process.env.ASTERWEFT_WEB_HOST?.trim() || '127.0.0.1';
const webPort = Number.parseInt(
  process.env.ASTERWEFT_WEB_PORT?.trim() || process.env.PORT?.trim() || '3000',
  10,
);

export default defineConfig(({ command }) => ({
  build: {
    minify: true,
  },
  cacheDir: isE2E ? 'node_modules/.vite-e2e' : 'node_modules/.vite',
  optimizeDeps: {
    include: [
      'html-to-image',
      'react',
      'react-dom',
      'react/jsx-dev-runtime',
      'react/jsx-runtime',
      'react-router',
      'react-router/dom',
    ],
  },
  plugins: [tailwindcss(), reactRouter()],
  resolve: {
    dedupe: ['react', 'react-dom'],
    tsconfigPaths: true,
  },
  server: {
    host: webHost,
    port: webPort,
    strictPort: true,
  },
  ssr: {
    ...(command === 'build'
      ? {
          external: [...serverRuntimeDependencies],
          noExternal: true,
        }
      : {}),
    optimizeDeps: {
      include: [
        'react',
        'react-dom',
        'react/jsx-dev-runtime',
        'react/jsx-runtime',
      ],
    },
  },
}));
