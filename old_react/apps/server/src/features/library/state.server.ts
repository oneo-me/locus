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
import { logWarn } from '~/logging';

export { normalizeLibraryState } from '@locus/document-library/contracts/library-state';

let loggedReadError: string | undefined;

function resolveLibraryStatePath(): string {
  return join(resolveDataDirectory(), 'state.json');
}

export function readLibraryState(): LibraryState {
  const path = resolveLibraryStatePath();
  if (!existsSync(path)) {
    loggedReadError = undefined;
    return normalizeLibraryState(null);
  }
  try {
    const state = normalizeLibraryState(JSON.parse(readFileSync(path, 'utf8')));
    loggedReadError = undefined;
    return state;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (loggedReadError !== message) {
      logWarn('library.state.read_failed', error);
      loggedReadError = message;
    }
    return normalizeLibraryState(null);
  }
}

function writeLibraryState(state: LibraryState): LibraryState {
  const normalized = normalizeLibraryState(state);
  const path = join(ensureDataDirectory(), 'state.json');
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
): LibraryState {
  const current = readLibraryState();
  return writeLibraryState({
    ...current,
    order: {
      ...current.order,
      ...Object.fromEntries(items.map((item) => [item.path, item.order])),
    },
  });
}

export function updateDocumentGraphParticipation(
  path: string,
  participates: boolean,
): LibraryState {
  const current = readLibraryState();
  return writeLibraryState({
    ...current,
    graphParticipation: {
      ...current.graphParticipation,
      [path]: participates,
    },
  });
}
