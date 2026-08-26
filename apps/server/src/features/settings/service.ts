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
import { closeLibrary, getLibrary } from '~/features/library/instance.server';
import { readLibraryNavigation } from '~/features/library/navigation';
import { logInfo } from '~/logging';
import { accessSync, constants, statSync } from 'node:fs';
import { isAbsolute, normalize } from 'node:path';
import { resetTerminalSessions } from '~/features/terminal/service.server';

function containsControlCharacter(value: string): boolean {
  return Array.from(value).some((character) => {
    const code = character.charCodeAt(0);
    return code <= 31 || code === 127;
  });
}

export function getGlobalSettings(): GlobalSettings {
  return readGlobalSettings();
}

export async function updateGlobalSettings(
  input: UpdateGlobalSettingsInput,
): Promise<GlobalSettings> {
  if (!input || typeof input !== 'object') {
    throw new Error('设置内容无效。');
  }

  const next: UpdateGlobalSettingsInput = {};
  if (
    input.libraryPath !== undefined &&
    input.defaultDocumentPath !== undefined
  ) {
    throw new Error('不能同时更新文档库路径和默认笔记。');
  }
  if (input.libraryPath !== undefined) {
    if (typeof input.libraryPath !== 'string' || !input.libraryPath.trim()) {
      throw new Error('文档库路径不能为空。');
    }
    const libraryPath = normalize(input.libraryPath.trim());
    if (!isAbsolute(libraryPath)) {
      throw new Error('文档库路径必须是绝对路径。');
    }
    try {
      if (!statSync(libraryPath).isDirectory()) throw new Error();
      accessSync(libraryPath, constants.R_OK | constants.W_OK);
    } catch {
      throw new Error('文档库路径必须指向可读写的现有目录。');
    }
    next.libraryPath = libraryPath;
    next.defaultDocumentPath = null;
  }
  if (input.defaultDocumentPath !== undefined) {
    if (input.defaultDocumentPath === null) {
      next.defaultDocumentPath = null;
    } else {
      if (
        typeof input.defaultDocumentPath !== 'string' ||
        !input.defaultDocumentPath.trim()
      ) {
        throw new Error('默认笔记路径必须是非空字符串或 null。');
      }
      const defaultDocumentPath = input.defaultDocumentPath.trim();
      const library = await getLibrary();
      const document = readLibraryNavigation(library).documents.find(
        (candidate) => candidate.path === defaultDocumentPath,
      );
      if (!document || document.directory.includes('/')) {
        throw new Error('默认笔记必须是未归档的顶级笔记。');
      }
      next.defaultDocumentPath = defaultDocumentPath;
    }
  }
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
  if (input.commandPaletteDefaultExpanded !== undefined) {
    if (typeof input.commandPaletteDefaultExpanded !== 'boolean') {
      throw new Error('commandPaletteDefaultExpanded 必须是布尔值。');
    }
    next.commandPaletteDefaultExpanded = input.commandPaletteDefaultExpanded;
  }
  if (input.mcpEnabled !== undefined) {
    if (typeof input.mcpEnabled !== 'boolean') {
      throw new Error('mcpEnabled 必须是布尔值。');
    }
    next.mcpEnabled = input.mcpEnabled;
  }
  if (input.showGraphAttachments !== undefined) {
    if (typeof input.showGraphAttachments !== 'boolean') {
      throw new Error('showGraphAttachments 必须是布尔值。');
    }
    next.showGraphAttachments = input.showGraphAttachments;
  }
  if (input.taskDraft !== undefined) {
    if (!input.taskDraft || typeof input.taskDraft !== 'object') {
      throw new Error('taskDraft 必须是对象。');
    }
    const taskDraft = input.taskDraft as unknown as Record<string, unknown>;
    for (const field of ['modelId', 'projectId', 'providerId'] as const) {
      const value = taskDraft[field];
      if (
        value !== null &&
        (typeof value !== 'string' ||
          !value.trim() ||
          value.length > 1024 ||
          containsControlCharacter(value))
      ) {
        throw new Error(`taskDraft.${field} 必须是有效标识或 null。`);
      }
    }
    next.taskDraft = {
      modelId:
        typeof taskDraft.modelId === 'string' ? taskDraft.modelId.trim() : null,
      projectId:
        typeof taskDraft.projectId === 'string'
          ? taskDraft.projectId.trim()
          : null,
      providerId:
        typeof taskDraft.providerId === 'string'
          ? taskDraft.providerId.trim()
          : null,
    };
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
  if (input.terminalShell !== undefined) {
    if (
      input.terminalShell !== null &&
      (typeof input.terminalShell !== 'string' ||
        !input.terminalShell.trim() ||
        input.terminalShell.length > 1024 ||
        containsControlCharacter(input.terminalShell))
    ) {
      throw new Error('terminalShell 必须是单行 shell 命令或 null。');
    }
    next.terminalShell = input.terminalShell?.trim() || null;
  }
  if (input.terminalStartupCommand !== undefined) {
    if (
      typeof input.terminalStartupCommand !== 'string' ||
      input.terminalStartupCommand.length > 4096 ||
      containsControlCharacter(input.terminalStartupCommand)
    ) {
      throw new Error(
        'terminalStartupCommand 必须是不超过 4096 字符的单行命令。',
      );
    }
    next.terminalStartupCommand = input.terminalStartupCommand.trim();
  }
  if (input.terminalTheme !== undefined) {
    if (
      input.terminalTheme !== 'auto' &&
      input.terminalTheme !== 'light' &&
      input.terminalTheme !== 'dark'
    ) {
      throw new Error('terminalTheme 必须是 auto、light 或 dark。');
    }
    next.terminalTheme = input.terminalTheme;
  }
  if (Object.keys(next).length === 0) {
    throw new Error('至少需要提供一个要更新的字段。');
  }
  const settings = writeGlobalSettings(next);
  if (
    next.libraryPath !== undefined ||
    next.terminalShell !== undefined ||
    next.terminalStartupCommand !== undefined
  ) {
    resetTerminalSessions();
  }
  if (next.libraryPath !== undefined) await closeLibrary();
  logInfo('settings.updated', { fields: Object.keys(next).sort() });
  return settings;
}
