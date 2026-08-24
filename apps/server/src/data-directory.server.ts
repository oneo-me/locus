import { mkdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { resolve } from 'node:path';

function defaultHome(): string {
  return process.env.LOCUS_HOME?.trim() || homedir();
}

export function resolveDataDirectory(home = defaultHome()): string {
  return resolve(home, '.oneo', 'locus');
}

export function ensureDataDirectory(home?: string): string {
  const directory = resolveDataDirectory(home);
  mkdirSync(directory, { recursive: true });
  return directory;
}
