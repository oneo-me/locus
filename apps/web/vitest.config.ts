import { resolve } from 'node:path';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  resolve: {
    alias: {
      '~': resolve(import.meta.dirname, 'app'),
    },
  },
  test: {
    environment: 'node',
    include: ['app/**/*.{test,spec}.{ts,tsx}'],
    pool: 'forks',
    fileParallelism: false,
  },
});
