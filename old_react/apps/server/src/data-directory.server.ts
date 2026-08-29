import { mkdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { resolve } from 'node:path';

export function resolveDataDirectory(): string {
  return resolve(homedir(), '.oneo', 'locus');
}

export function ensureDataDirectory(): string {
  const directory = resolveDataDirectory();
  mkdirSync(directory, { recursive: true });
  return directory;
}
