import {
  isBrowserViewableAsset,
  isPreviewableImageAsset,
  parseAssetPath,
} from '@locus/document-library/contracts/asset';
import { createAssetResponse } from '~/features/asset/response';
import {
  ensureAssetThumbnailCacheDir,
  readAssetThumbnail,
} from '~/features/asset/thumbnail.server';
import { isDocumentIconName } from '@locus/document-library/contracts/document-icon';
import type { DocumentLibrary } from '@locus/document-library/server/document-library';
import type { DocumentAsset } from '@locus/document-library/contracts/library-types';
import { getLibrary } from '~/features/library/instance.server';
import { urlPathToDocumentPath } from '@locus/document-library/contracts/library-route';

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
