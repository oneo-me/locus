import {
  type GlobalSettings,
  readGlobalSettings,
  writeGlobalSettings,
} from './global-settings.server';
import {
  MAX_WORKSPACE_SIDEBAR_WIDTH,
  MIN_WORKSPACE_SIDEBAR_WIDTH,
  type UpdateGlobalSettingsInput,
} from '@locus/document-library/contracts/global-settings';

export function getGlobalSettings(): GlobalSettings {
  return readGlobalSettings();
}

export function updateGlobalSettings(
  input: UpdateGlobalSettingsInput,
): GlobalSettings {
  if (!input || typeof input !== 'object') {
    throw new Error('设置内容无效。');
  }

  const next: UpdateGlobalSettingsInput = {};
  if (input.theme !== undefined) {
    if (
      input.theme !== 'light' &&
      input.theme !== 'dark' &&
      input.theme !== 'system'
    ) {
      throw new Error('theme 必须是 light、dark 或 system。');
    }
    next.theme = input.theme;
  }
  if (input.mcpEnabled !== undefined) {
    if (typeof input.mcpEnabled !== 'boolean') {
      throw new Error('mcpEnabled 必须是布尔值。');
    }
    next.mcpEnabled = input.mcpEnabled;
  }
  if (input.workspaceSidebarWidth !== undefined) {
    if (
      typeof input.workspaceSidebarWidth !== 'number' ||
      !Number.isInteger(input.workspaceSidebarWidth) ||
      input.workspaceSidebarWidth < MIN_WORKSPACE_SIDEBAR_WIDTH ||
      input.workspaceSidebarWidth > MAX_WORKSPACE_SIDEBAR_WIDTH
    ) {
      throw new Error(
        `workspaceSidebarWidth 必须是 ${MIN_WORKSPACE_SIDEBAR_WIDTH} 到 ${MAX_WORKSPACE_SIDEBAR_WIDTH} 之间的整数。`,
      );
    }
    next.workspaceSidebarWidth = input.workspaceSidebarWidth;
  }
  if (Object.keys(next).length === 0) {
    throw new Error('至少需要提供一个要更新的字段。');
  }
  return writeGlobalSettings(next);
}
