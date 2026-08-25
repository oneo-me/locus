import { bytesFromBase64, bytesToBase64 } from '~/lib/base64';
import { logInfo } from '~/logging';
import { execFile } from 'node:child_process';
import { dirname } from 'node:path';
import { promisify } from 'node:util';
import type {
  LibrarySelectionSourceView,
  LibrarySelectionViewInput,
  OpenDocumentInput,
} from '@locus/document-library/contracts/library-api';
import { resolveExistingDocument } from '@locus/document-library/server/paths';
import { isPreviewableImageAsset } from '@locus/document-library/contracts/asset';
import {
  ensureAssetThumbnailCacheDir,
  readAssetThumbnail,
} from '../asset/thumbnail.server';
import { maxDocumentIconBytes } from '@locus/document-library/contracts/document-icon';
import type {
  DocumentContent,
  ImageSize,
} from '@locus/document-library/contracts/library-types';
import { getLibrary } from './instance.server';
import { readLibraryNavigation } from './navigation';
import { readLibraryRepositoryView } from './repository-view.server';
import { resolveLibrarySelection } from './selection.server';
import { readLibraryState, updateLibraryDocumentOrder } from './state.server';
import { updateDocumentGraphParticipation } from './state.server';
import { excludedDocumentGraphPaths } from '@locus/document-library/contracts/library-state';

const execFileAsync = promisify(execFile);

async function readAssetImageSizes(
  library: Awaited<ReturnType<typeof getLibrary>>,
  selected: DocumentContent | null,
): Promise<Record<string, ImageSize>> {
  if (!selected) return {};
  const cacheDir = ensureAssetThumbnailCacheDir();
  const entries = await Promise.all(
    selected.assets.filter(isPreviewableImageAsset).map(async (name) => {
      try {
        const thumbnail = await readAssetThumbnail(
          library.readAsset(selected.path, name),
          cacheDir,
        );
        return [
          name,
          { height: thumbnail.height, width: thumbnail.width },
        ] as const;
      } catch {
        return null;
      }
    }),
  );
  return Object.fromEntries(entries.filter((entry) => entry !== null));
}

async function readSelectionView(
  library: Awaited<ReturnType<typeof getLibrary>>,
  requestedPath: string | undefined,
): Promise<LibrarySelectionSourceView> {
  const selection = resolveLibrarySelection(library, requestedPath);
  const status = library.status();
  const assetImageSizes = await readAssetImageSizes(
    library,
    selection.selected,
  );
  return {
    ...selection,
    assetImageSizes,
    indexedAt: status.indexedAt,
  };
}

export async function getLibraryWorkspaceView() {
  const library = await getLibrary();
  const libraryState = readLibraryState();
  const navigation = readLibraryNavigation(library);
  const repositoryView = readLibraryRepositoryView(library);

  return {
    status: library.status(),
    ...navigation,
    repositoryChanges: repositoryView.changes,
    repositoryError: repositoryView.error,
    repositoryRevision: repositoryView.revision,
    libraryState,
  };
}

export async function getLibraryArchiveView() {
  const library = await getLibrary();
  return {
    documents: library
      .listAllDocuments()
      .filter((document) => document.archived)
      .map((document) => ({
        attachmentCount: document.assets.length,
        directory: document.directory,
        icon: document.icon,
        iconImageModifiedAt: document.iconImageModifiedAt,
        path: document.path,
        title: document.title,
      })),
  };
}

export async function getLibraryGraph() {
  const library = await getLibrary();
  const state = readLibraryState();
  const documents = library.listAllDocuments();
  return {
    status: library.status(),
    graph: library.getDocumentGraph(
      excludedDocumentGraphPaths(state, documents),
    ),
  };
}

export async function getLibrarySelection(input: LibrarySelectionViewInput) {
  if (!input || typeof input.doc !== 'string' || !input.doc.trim()) {
    throw new Error('doc 必须是非空字符串。');
  }
  const documentPath = input.doc.trim();
  const library = await getLibrary();
  return readSelectionView(library, documentPath);
}

export async function openDocument(input: OpenDocumentInput) {
  if (!input || typeof input.path !== 'string' || !input.path.trim()) {
    throw new Error('path 必须是非空字符串。');
  }
  if (input.target !== 'directory' && input.target !== 'vscode') {
    throw new Error('target 必须是 directory 或 vscode。');
  }

  const library = await getLibrary();
  const document = library.getDocument(input.path);
  const file = resolveExistingDocument(
    library.status().rootPath,
    document.path,
  );
  const target = input.target === 'directory' ? dirname(file) : file;

  if (process.platform === 'darwin') {
    await execFileAsync(
      'open',
      input.target === 'vscode'
        ? ['-a', 'Visual Studio Code', target]
        : [target],
    );
  } else if (process.platform === 'win32') {
    await execFileAsync(
      input.target === 'vscode' ? 'code.cmd' : 'explorer.exe',
      [target],
    );
  } else {
    await execFileAsync(input.target === 'vscode' ? 'code' : 'xdg-open', [
      target,
    ]);
  }

  logInfo('library.document.opened', {
    path: document.path,
    target: input.target,
  });
  return { ok: true as const };
}

export async function openLibraryTerminal() {
  if (process.platform !== 'darwin') {
    throw new Error('当前只支持在 macOS 上使用 Ghostty 打开文档库。');
  }

  const library = await getLibrary();
  await execFileAsync('open', ['-a', 'Ghostty', library.status().rootPath]);

  logInfo('library.terminal.opened', { terminal: 'ghostty' });
  return { ok: true as const };
}

export interface UpdateDocumentIconImageInput {
  path: string;
  dataBase64: string | null;
}

export async function updateDocumentIconImage(
  input: UpdateDocumentIconImageInput,
) {
  if (!input || typeof input.path !== 'string' || !input.path.trim()) {
    throw new Error('path 必须是非空字符串。');
  }
  if (input.dataBase64 !== null && typeof input.dataBase64 !== 'string') {
    throw new Error('dataBase64 必须是字符串或 null。');
  }
  if (
    input.dataBase64 !== null &&
    input.dataBase64.length > Math.ceil((maxDocumentIconBytes * 4) / 3) + 4
  ) {
    throw new Error('文档图标不能超过 256 KiB。');
  }

  const library = await getLibrary();
  if (input.dataBase64 === null) {
    const result = library.deleteDocumentIcon(input.path);
    logInfo('library.document_icon.deleted', { path: input.path });
    return result;
  }

  const bytes = bytesFromBase64(input.dataBase64);
  if (bytesToBase64(bytes) !== input.dataBase64) {
    throw new Error('dataBase64 必须是规范的 Base64 字符串。');
  }
  const result = library.writeDocumentIcon(input.path, bytes);
  logInfo('library.document_icon.updated', { path: input.path });
  return result;
}

export interface ReorderDocumentsInput {
  items: Array<{ path: string; order: number }>;
}

export async function reorderDocuments(input: ReorderDocumentsInput) {
  if (!input || !Array.isArray(input.items) || input.items.length === 0) {
    throw new Error('items 不能为空。');
  }
  const items = input.items.map((item) => {
    if (!item || typeof item.path !== 'string') {
      throw new Error('path 必须是字符串。');
    }
    if (typeof item.order !== 'number' || !Number.isFinite(item.order)) {
      throw new Error('order 必须是有限数字。');
    }
    return { path: item.path, order: item.order };
  });
  const library = await getLibrary();
  for (const item of items) library.getDocument(item.path);
  updateLibraryDocumentOrder(items);
  logInfo('library.documents.reordered', { count: items.length });
  return { ok: true as const };
}

export interface SetDocumentGraphParticipationInput {
  path: string;
  participates: boolean;
}

export async function setDocumentGraphParticipation(
  input: SetDocumentGraphParticipationInput,
) {
  if (!input || typeof input.path !== 'string' || !input.path.trim()) {
    throw new Error('path 必须是非空字符串。');
  }
  if (typeof input.participates !== 'boolean') {
    throw new Error('participates 必须是布尔值。');
  }
  const path = input.path.trim();
  const library = await getLibrary();
  library.getDocument(path);
  const state = updateDocumentGraphParticipation(path, input.participates);
  return {
    ok: true as const,
    participates: state.graphParticipation[path] ?? true,
  };
}

export interface SetDocumentArchivedInput {
  path: string;
  archived: boolean;
}

export async function setDocumentArchived(input: SetDocumentArchivedInput) {
  if (!input || typeof input.path !== 'string' || !input.path.trim()) {
    throw new Error('path 必须是非空字符串。');
  }
  if (typeof input.archived !== 'boolean') {
    throw new Error('archived 必须是布尔值。');
  }
  const library = await getLibrary();
  const document = library.getDocument(input.path.trim());
  const updated = library.setDocumentArchived(document.path, input.archived);
  logInfo(
    input.archived
      ? 'library.document.archived'
      : 'library.document.unarchived',
    {
      path: document.path,
    },
  );
  return {
    ok: true as const,
    archived: updated.archived,
  };
}
