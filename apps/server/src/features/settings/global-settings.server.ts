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
import { logWarn } from '~/logging';

export type {
  GlobalSettings,
  GlobalTheme,
} from '@locus/document-library/contracts/global-settings';
export { DEFAULT_GLOBAL_SETTINGS } from '@locus/document-library/contracts/global-settings';

let loggedReadError: string | undefined;

function resolveSettingsPath(): string {
  return join(resolveDataDirectory(), 'settings.json');
}

export function readGlobalSettings(): GlobalSettings {
  const settingsPath = resolveSettingsPath();
  if (!existsSync(settingsPath)) {
    loggedReadError = undefined;
    return normalizeGlobalSettings(null);
  }

  try {
    const raw = readFileSync(settingsPath, 'utf8');
    const settings = normalizeGlobalSettings(JSON.parse(raw) as unknown);
    loggedReadError = undefined;
    return settings;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (loggedReadError !== message) {
      logWarn('settings.read_failed', error);
      loggedReadError = message;
    }
    return normalizeGlobalSettings(null);
  }
}

export function writeGlobalSettings(
  patch: Partial<GlobalSettings>,
): GlobalSettings {
  const settingsPath = resolveSettingsPath();
  ensureDataDirectory();

  const next = normalizeGlobalSettings({
    ...readGlobalSettings(),
    ...patch,
  });
  writeFileSync(settingsPath, `${JSON.stringify(next, null, 2)}\n`, 'utf8');
  return next;
}
