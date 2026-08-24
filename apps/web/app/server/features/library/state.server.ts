import {
  existsSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { basename, dirname, join } from 'node:path';
import {
  currentEnvironment,
  ensureLibraryStatePath,
  resolveLibraryStatePath,
  type RuntimeEnvironment,
} from '~/server/features/settings/global-paths.server';
import { type LibraryState, normalizeLibraryState } from './state';

export { normalizeLibraryState } from './state';

export function readLibraryState(
  environment: RuntimeEnvironment = currentEnvironment(),
  home?: string,
): LibraryState {
  const path = resolveLibraryStatePath(environment, home);
  if (!existsSync(path)) return normalizeLibraryState(null);
  try {
    return normalizeLibraryState(JSON.parse(readFileSync(path, 'utf8')));
  } catch {
    return normalizeLibraryState(null);
  }
}

function writeLibraryState(
  state: LibraryState,
  environment: RuntimeEnvironment = currentEnvironment(),
  home?: string,
): LibraryState {
  const normalized = normalizeLibraryState(state);
  const path = ensureLibraryStatePath(environment, home);
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
  environment: RuntimeEnvironment = currentEnvironment(),
  home?: string,
): LibraryState {
  const current = readLibraryState(environment, home);
  return writeLibraryState(
    {
      ...current,
      order: {
        ...current.order,
        ...Object.fromEntries(items.map((item) => [item.path, item.order])),
      },
    },
    environment,
    home,
  );
}

export function updateDocumentGraphParticipation(
  path: string,
  participates: boolean,
  environment: RuntimeEnvironment = currentEnvironment(),
  home?: string,
): LibraryState {
  const current = readLibraryState(environment, home);
  return writeLibraryState(
    {
      ...current,
      graphParticipation: {
        ...current.graphParticipation,
        [path]: participates,
      },
    },
    environment,
    home,
  );
}
