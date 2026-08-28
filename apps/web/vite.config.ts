import { reactRouter } from '@react-router/dev/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [tailwindcss(), reactRouter()],
  optimizeDeps: {
    include: [
      '@base-ui/react > @base-ui/utils > use-sync-external-store/shim',
      '@base-ui/react > @base-ui/utils > use-sync-external-store/shim/with-selector',
    ],
  },
  resolve: {
    tsconfigPaths: true,
  },
  server: {
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:3001',
        ws: true,
      },
      '/mcp': 'http://127.0.0.1:3001',
    },
  },
});
