import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  ensureDataDirectory,
  resolveDataDirectory,
} from '~/data-directory.server';
import {
  type GlobalSettings,
  normalizeGlobalSettings,
} from '@locus/document-library/contracts/global-settings';

export type {
  GlobalSettings,
  GlobalTheme,
} from '@locus/document-library/contracts/global-settings';
export { DEFAULT_GLOBAL_SETTINGS } from '@locus/document-library/contracts/global-settings';

function resolveSettingsPath(home?: string): string {
  return join(resolveDataDirectory(home), 'settings.json');
}

export function readGlobalSettings(home?: string): GlobalSettings {
  const settingsPath = resolveSettingsPath(home);
  if (!existsSync(settingsPath)) {
    return normalizeGlobalSettings(null);
  }

  try {
    const raw = readFileSync(settingsPath, 'utf8');
    return normalizeGlobalSettings(JSON.parse(raw) as unknown);
  } catch {
    return normalizeGlobalSettings(null);
  }
}

export function writeGlobalSettings(
  patch: Partial<GlobalSettings>,
  home?: string,
): GlobalSettings {
  const settingsPath = resolveSettingsPath(home);
  ensureDataDirectory(home);

  const next = normalizeGlobalSettings({
    ...readGlobalSettings(home),
    ...patch,
  });
  writeFileSync(settingsPath, `${JSON.stringify(next, null, 2)}\n`, 'utf8');
  return next;
}
