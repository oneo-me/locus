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
  if (next.libraryPath !== undefined) await closeLibrary();
  const settings = writeGlobalSettings(next);
  logInfo('settings.updated', { fields: Object.keys(next).sort() });
  return settings;
}
