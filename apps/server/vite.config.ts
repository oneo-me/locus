import { fileURLToPath } from 'node:url';

const serverDirectory = fileURLToPath(new URL('.', import.meta.url));

export default {
  root: serverDirectory,
  resolve: {
    alias: {
      '~': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  build: {
    minify: true,
    outDir: 'dist',
    ssr: 'src/index.ts',
    target: 'node22',
    rollupOptions: {
      output: {
        entryFileNames: 'server.js',
      },
    },
  },
};
