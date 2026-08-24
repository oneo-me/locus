import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, describe, expect, test } from 'vitest';
import {
  resolveAssetThumbnailCacheDir,
  resolveGlobalConfigDir,
  resolveGlobalSettingsPath,
  resolveLibraryCachePath,
  resolveLibraryStatePath,
} from './global-paths.server';

const homes: string[] = [];

function temporaryHome(): string {
  const home = mkdtempSync(join(tmpdir(), 'asterweft-home-'));
  homes.push(home);
  return home;
}

afterEach(() => {
  for (const home of homes.splice(0)) {
    rmSync(home, { recursive: true, force: true });
  }
});

describe('global paths', () => {
  test('uses the base config directory in production', () => {
    const home = temporaryHome();

    expect(resolveGlobalConfigDir('production', home)).toBe(
      join(home, '.oneo', 'asterweft'),
    );
    expect(resolveGlobalSettingsPath('production', home)).toBe(
      join(home, '.oneo', 'asterweft', 'settings.json'),
    );
  });

  test('uses a _dev config directory in development', () => {
    const home = temporaryHome();

    expect(resolveGlobalConfigDir('development', home)).toBe(
      join(home, '.oneo', 'asterweft_dev'),
    );
    expect(resolveGlobalSettingsPath('development', home)).toBe(
      join(home, '.oneo', 'asterweft_dev', 'settings.json'),
    );
  });

  test('uses the same cache filename under each environment directory', () => {
    const home = temporaryHome();
    expect(resolveLibraryCachePath('production', home)).toBe(
      join(home, '.oneo', 'asterweft', 'cache.sqlite'),
    );
    expect(resolveLibraryCachePath('development', home)).toBe(
      join(home, '.oneo', 'asterweft_dev', 'cache.sqlite'),
    );
  });

  test('stores generated image thumbnails in the global config directory', () => {
    const home = temporaryHome();
    expect(resolveAssetThumbnailCacheDir('development', home)).toBe(
      join(home, '.oneo', 'asterweft_dev', 'thumbnails'),
    );
  });

  test('uses a fixed state path in the global config directory', () => {
    const home = temporaryHome();
    expect(resolveLibraryStatePath('development', home)).toBe(
      join(home, '.oneo', 'asterweft_dev', 'state.json'),
    );
  });
});
