import {
  existsSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { basename, dirname, join } from 'node:path';
import {
  ensureDataDirectory,
  resolveDataDirectory,
} from '~/data-directory.server';
import {
  type LibraryState,
  normalizeLibraryState,
} from '@locus/document-library/contracts/library-state';

export { normalizeLibraryState } from '@locus/document-library/contracts/library-state';

function resolveLibraryStatePath(home?: string): string {
  return join(resolveDataDirectory(home), 'state.json');
}

export function readLibraryState(home?: string): LibraryState {
  const path = resolveLibraryStatePath(home);
  if (!existsSync(path)) return normalizeLibraryState(null);
  try {
    return normalizeLibraryState(JSON.parse(readFileSync(path, 'utf8')));
  } catch {
    return normalizeLibraryState(null);
  }
}

function writeLibraryState(state: LibraryState, home?: string): LibraryState {
  const normalized = normalizeLibraryState(state);
  const path = join(ensureDataDirectory(home), 'state.json');
  const temporaryPath = join(
    dirname(path),
    `.${basename(path)}.${crypto.randomUUID()}.tmp`,
  );
  try {
    writeFileSync(temporaryPath, `${JSON.stringify(normalized, null, 2)}\n`, {
      encoding: 'utf8',
      mode: 0o600,
    });
    renameSync(temporaryPath, path);
  } finally {
    if (existsSync(temporaryPath)) rmSync(temporaryPath);
  }
  return normalized;
}

export function updateLibraryDocumentOrder(
  items: Array<{ path: string; order: number }>,
  home?: string,
): LibraryState {
  const current = readLibraryState(home);
  return writeLibraryState(
    {
      ...current,
      order: {
        ...current.order,
        ...Object.fromEntries(items.map((item) => [item.path, item.order])),
      },
    },
    home,
  );
}
