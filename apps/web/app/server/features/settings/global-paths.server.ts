import { mkdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';

export type RuntimeEnvironment = 'development' | 'production';

export function currentEnvironment(): RuntimeEnvironment {
  return process.env.NODE_ENV === 'production' ? 'production' : 'development';
}

function defaultHome(): string {
  return process.env.ASTERWEFT_HOME?.trim() || homedir();
}

function configDirectoryName(environment: RuntimeEnvironment): string {
  return environment === 'development' ? 'asterweft_dev' : 'asterweft';
}

export function resolveGlobalConfigDir(
  environment: RuntimeEnvironment = currentEnvironment(),
  home = defaultHome(),
): string {
  return resolve(home, '.oneo', configDirectoryName(environment));
}

export function ensureGlobalConfigDir(
  environment: RuntimeEnvironment = currentEnvironment(),
  home = defaultHome(),
): string {
  const configDir = resolveGlobalConfigDir(environment, home);
  mkdirSync(configDir, { recursive: true });
  return configDir;
}

export function resolveGlobalSettingsPath(
  environment: RuntimeEnvironment = currentEnvironment(),
  home = defaultHome(),
): string {
  return join(resolveGlobalConfigDir(environment, home), 'settings.json');
}

export function resolveLibraryCachePath(
  environment: RuntimeEnvironment = currentEnvironment(),
  home = defaultHome(),
): string {
  return join(resolveGlobalConfigDir(environment, home), 'cache.sqlite');
}

export function ensureLibraryCachePath(
  environment: RuntimeEnvironment = currentEnvironment(),
  home = defaultHome(),
): string {
  const cachePath = resolveLibraryCachePath(environment, home);
  mkdirSync(resolveGlobalConfigDir(environment, home), { recursive: true });
  return cachePath;
}

export function resolveAssetThumbnailCacheDir(
  environment: RuntimeEnvironment = currentEnvironment(),
  home = defaultHome(),
): string {
  return join(resolveGlobalConfigDir(environment, home), 'thumbnails');
}

export function ensureAssetThumbnailCacheDir(
  environment: RuntimeEnvironment = currentEnvironment(),
  home = defaultHome(),
): string {
  const cacheDir = resolveAssetThumbnailCacheDir(environment, home);
  mkdirSync(cacheDir, { recursive: true });
  return cacheDir;
}

export function resolveLibraryStatePath(
  environment: RuntimeEnvironment = currentEnvironment(),
  home = defaultHome(),
): string {
  return join(resolveGlobalConfigDir(environment, home), 'state.json');
}

export function ensureLibraryStatePath(
  environment: RuntimeEnvironment = currentEnvironment(),
  home = defaultHome(),
): string {
  const statePath = resolveLibraryStatePath(environment, home);
  mkdirSync(resolveGlobalConfigDir(environment, home), { recursive: true });
  return statePath;
}
