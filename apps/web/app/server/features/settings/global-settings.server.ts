import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import {
  currentEnvironment,
  ensureGlobalConfigDir,
  resolveGlobalSettingsPath,
  type RuntimeEnvironment,
} from './global-paths.server';
import {
  type GlobalSettings,
  normalizeGlobalSettings,
} from './global-settings-types';

export type { GlobalSettings, GlobalTheme } from './global-settings-types';
export { DEFAULT_GLOBAL_SETTINGS } from './global-settings-types';

export function readGlobalSettings(
  environment: RuntimeEnvironment = currentEnvironment(),
  home?: string,
): GlobalSettings {
  const settingsPath = resolveGlobalSettingsPath(environment, home);
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
  environment: RuntimeEnvironment = currentEnvironment(),
  home?: string,
): GlobalSettings {
  const settingsPath = resolveGlobalSettingsPath(environment, home);
  ensureGlobalConfigDir(environment, home);

  const next = normalizeGlobalSettings({
    ...readGlobalSettings(environment, home),
    ...patch,
  });
  writeFileSync(settingsPath, `${JSON.stringify(next, null, 2)}\n`, 'utf8');
  return next;
}
