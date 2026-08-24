import { existsSync } from 'node:fs';
import { resolve } from 'node:path';

/**
 * Load monorepo-root `.env` into `process.env` without overriding existing vars.
 * Bun did this automatically; Node does not.
 */
export function loadRootEnvFile(): void {
  const envPath = resolve(import.meta.dirname, '../../../.env');
  if (!existsSync(envPath)) return;
  process.loadEnvFile(envPath);
}
