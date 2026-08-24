import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, describe, expect, test } from 'vitest';
import { resolveGlobalSettingsPath } from './global-paths.server';
import {
  DEFAULT_GLOBAL_SETTINGS,
  readGlobalSettings,
  writeGlobalSettings,
} from './global-settings.server';

const homes: string[] = [];

function temporaryHome(): string {
  const home = mkdtempSync(join(tmpdir(), 'asterweft-settings-'));
  homes.push(home);
  return home;
}

afterEach(() => {
  for (const home of homes.splice(0)) {
    rmSync(home, { recursive: true, force: true });
  }
});

describe('global settings', () => {
  test('returns the default theme when settings are missing', () => {
    const home = temporaryHome();

    expect(readGlobalSettings('development', home)).toEqual(
      DEFAULT_GLOBAL_SETTINGS,
    );
  });

  test('persists theme updates to settings.json', () => {
    const home = temporaryHome();

    expect(writeGlobalSettings({ theme: 'dark' }, 'development', home)).toEqual(
      {
        theme: 'dark',
        showGraphAttachments: true,
        mcpEnabled: false,
        workspaceSidebarWidth: 284,
      },
    );
    expect(readGlobalSettings('development', home)).toEqual({
      theme: 'dark',
      showGraphAttachments: true,
      mcpEnabled: false,
      workspaceSidebarWidth: 284,
    });
    expect(
      JSON.parse(
        readFileSync(resolveGlobalSettingsPath('development', home), 'utf8'),
      ),
    ).toEqual({
      theme: 'dark',
      showGraphAttachments: true,
      mcpEnabled: false,
      workspaceSidebarWidth: 284,
    });
  });

  test('persists graph attachment visibility', () => {
    const home = temporaryHome();

    expect(
      writeGlobalSettings({ showGraphAttachments: false }, 'development', home),
    ).toEqual({
      theme: 'system',
      showGraphAttachments: false,
      mcpEnabled: false,
      workspaceSidebarWidth: 284,
    });
    expect(readGlobalSettings('development', home).showGraphAttachments).toBe(
      false,
    );
  });

  test('persists the MCP server switch', () => {
    const home = temporaryHome();

    expect(
      writeGlobalSettings({ mcpEnabled: true }, 'development', home),
    ).toEqual({
      theme: 'system',
      showGraphAttachments: true,
      mcpEnabled: true,
      workspaceSidebarWidth: 284,
    });
    expect(readGlobalSettings('development', home).mcpEnabled).toBe(true);
  });

  test('persists the workspace sidebar width', () => {
    const home = temporaryHome();

    expect(
      writeGlobalSettings({ workspaceSidebarWidth: 336 }, 'development', home),
    ).toMatchObject({ workspaceSidebarWidth: 336 });
    expect(readGlobalSettings('development', home).workspaceSidebarWidth).toBe(
      336,
    );
  });

  test('uses the default sidebar width as the persisted minimum', () => {
    const home = temporaryHome();
    const settingsPath = resolveGlobalSettingsPath('development', home);
    mkdirSync(join(home, '.oneo', 'asterweft_dev'), { recursive: true });
    writeFileSync(settingsPath, '{"workspaceSidebarWidth":220}\n', 'utf8');

    expect(readGlobalSettings('development', home).workspaceSidebarWidth).toBe(
      284,
    );
  });

  test('ignores invalid theme values and keeps the default', () => {
    const home = temporaryHome();
    const settingsPath = resolveGlobalSettingsPath('development', home);
    mkdirSync(join(home, '.oneo', 'asterweft_dev'), { recursive: true });
    writeFileSync(settingsPath, '{"theme":"neon"}\n', 'utf8');

    expect(readGlobalSettings('development', home)).toEqual(
      DEFAULT_GLOBAL_SETTINGS,
    );
  });

  test('does not read settings from the old config directory', () => {
    const home = temporaryHome();
    mkdirSync(join(home, '.asterweft'), { recursive: true });
    writeFileSync(
      join(home, '.asterweft', 'settings.json'),
      '{"theme":"dark","showGraphAttachments":false}\n',
      'utf8',
    );

    expect(readGlobalSettings('development', home)).toEqual(
      DEFAULT_GLOBAL_SETTINGS,
    );
  });
});
