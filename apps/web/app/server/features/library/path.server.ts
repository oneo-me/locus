import { loadRootEnvFile } from '~/server/env.server';
import { existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

function findProjectDocumentLibrary(startPath: string): string | null {
  let directory = resolve(startPath);

  while (true) {
    const libraryPath = resolve(directory, 'docs');
    if (existsSync(resolve(libraryPath, 'policy.md'))) return libraryPath;

    const parent = dirname(directory);
    if (parent === directory) return null;
    directory = parent;
  }
}

loadRootEnvFile();

export function resolveLibraryPath(): string {
  const launchPath = process.cwd();
  const configuredPath = process.env.ASTERWEFT_LIBRARY_PATH?.trim();

  if (configuredPath) return resolve(launchPath, configuredPath);
  return findProjectDocumentLibrary(launchPath) ?? resolve(launchPath, 'docs');
}
