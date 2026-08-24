import {
  isBrowserViewableAsset,
  isPreviewableImageAsset,
  parseAssetPath,
} from '~/server/features/asset/protocol';
import { createAssetResponse } from '~/server/features/asset/response';
import { readAssetThumbnail } from '~/server/features/asset/thumbnail.server';
import { isDocumentIconName } from '~/server/features/library/core/document-icon';
import type { DocumentLibrary } from '~/server/features/library/core/document-library';
import type { DocumentAsset } from '~/server/features/library/core/types';
import { getLibrary } from '~/server/features/library/instance.server';
import { urlPathToDocumentPath } from '~/server/features/library/route';
import { ensureAssetThumbnailCacheDir } from '~/server/features/settings/global-paths.server';

export async function createRouteAssetResponse(
  request: Request,
  urlPath: string,
  providedLibrary?: DocumentLibrary,
  thumbnailCacheDir?: string,
): Promise<Response | null> {
  const library = providedLibrary ?? (await getLibrary());

  try {
    library.getDocument(urlPathToDocumentPath(urlPath));
    return null;
  } catch {
    // An exact article match always wins over an attachment with the same URL.
  }

  const target = parseAssetPath(urlPath);
  if (!target) return null;

  let asset: DocumentAsset;
  try {
    asset = isDocumentIconName(target.assetName)
      ? library.readDocumentIcon(target.documentPath)
      : library.readAsset(target.documentPath, target.assetName);
  } catch {
    return null;
  }

  const thumbnail = new URL(request.url).searchParams.get('thumbnail') === '1';
  if (thumbnail && !isPreviewableImageAsset(asset.name)) {
    return new Response('该附件无法生成图片缩略图。', { status: 415 });
  }
  const responseAsset = thumbnail
    ? await readAssetThumbnail(
        asset,
        thumbnailCacheDir ?? ensureAssetThumbnailCacheDir(),
      )
    : asset;
  return createAssetResponse(
    request,
    responseAsset,
    thumbnail || isBrowserViewableAsset(asset.name),
    thumbnail ? undefined : target.documentPath,
  );
}
