import { bytesFromBase64, bytesToBase64 } from '~/lib/base64';
import type {
  LibrarySelectionSourceView,
  LibrarySelectionViewInput,
  LibraryWorkspaceViewInput,
} from '@locus/document-library/contracts/library-api';
import { isPreviewableImageAsset } from '@locus/document-library/contracts/asset';
import { readAssetThumbnail } from '../asset/thumbnail.server';
import { maxDocumentIconBytes } from '@locus/document-library/contracts/document-icon';
import type {
  DocumentContent,
  ImageSize,
} from '@locus/document-library/contracts/library-types';
import { getLibrary } from './instance.server';
import { readLibraryNavigation } from './navigation';
import { readLibraryRepositoryView } from './repository-view.server';
import { resolveLibrarySelection } from './selection.server';
import { ensureAssetThumbnailCacheDir } from '../settings/global-paths.server';
import { readLibraryState, updateLibraryDocumentOrder } from './state.server';

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

export async function getLibraryWorkspaceView(
  input: LibraryWorkspaceViewInput,
) {
  const data = {
    query: typeof input.query === 'string' ? input.query.trim() : undefined,
  };
  const library = await getLibrary();
  const navigation = readLibraryNavigation(library, data.query);
  const repositoryView = readLibraryRepositoryView(library);

  return {
    status: library.status(),
    ...navigation,
    repositoryChanges: repositoryView.changes,
    repositoryError: repositoryView.error,
    repositoryRevision: repositoryView.revision,
    libraryState: readLibraryState(),
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

export interface UpdateDocumentMetadataInput {
  path: string;
  icon?: string | null;
}

export async function updateDocumentMetadata(
  input: UpdateDocumentMetadataInput,
) {
  if (!input || typeof input.path !== 'string') {
    throw new Error('path 必须是字符串。');
  }
  if (
    input.icon !== undefined &&
    input.icon !== null &&
    typeof input.icon !== 'string'
  ) {
    throw new Error('icon 必须是字符串。');
  }
  if (input.icon === undefined) throw new Error('必须提供 icon。');
  const library = await getLibrary();
  return library.updateDocument({ path: input.path, icon: input.icon });
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
    return library.deleteDocumentIcon(input.path);
  }

  const bytes = bytesFromBase64(input.dataBase64);
  if (bytesToBase64(bytes) !== input.dataBase64) {
    throw new Error('dataBase64 必须是规范的 Base64 字符串。');
  }
  return library.writeDocumentIcon(input.path, bytes);
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
  return { ok: true as const };
}
