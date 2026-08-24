import { documentIconName } from './document-icon.js';
import {
  documentPathToUrlPath,
  urlPathToDocumentPath,
} from './library-route.js';

const assetContentTypes = new Map([
  ['.avif', 'image/avif'],
  ['.bmp', 'image/bmp'],
  ['.csv', 'text/csv; charset=utf-8'],
  ['.gif', 'image/gif'],
  ['.html', 'text/html; charset=utf-8'],
  ['.jpeg', 'image/jpeg'],
  ['.jpg', 'image/jpeg'],
  ['.json', 'application/json; charset=utf-8'],
  ['.md', 'text/markdown; charset=utf-8'],
  ['.pdf', 'application/pdf'],
  ['.png', 'image/png'],
  ['.svg', 'image/svg+xml'],
  ['.txt', 'text/plain; charset=utf-8'],
  ['.webp', 'image/webp'],
  ['.zip', 'application/zip'],
]);

const previewableImageTypes = new Set([
  'image/avif',
  'image/bmp',
  'image/gif',
  'image/jpeg',
  'image/png',
  'image/svg+xml',
  'image/webp',
]);

function getAssetExtension(assetName: string): string {
  const extensionStart = assetName.lastIndexOf('.');
  return extensionStart < 0
    ? ''
    : assetName.slice(extensionStart).toLowerCase();
}

export function getAssetContentType(assetName: string): string {
  return (
    assetContentTypes.get(getAssetExtension(assetName)) ??
    'application/octet-stream'
  );
}

export function isPreviewableImageAsset(assetName: string): boolean {
  return previewableImageTypes.has(getAssetContentType(assetName));
}

export function isBrowserViewableAsset(assetName: string): boolean {
  const contentType = getAssetContentType(assetName);
  return (
    previewableImageTypes.has(contentType) ||
    contentType === 'application/pdf' ||
    contentType === 'text/html; charset=utf-8'
  );
}

export function getAssetUrl(documentPath: string, assetName: string): string {
  const documentUrlPath = documentPathToUrlPath(documentPath);
  const path = documentUrlPath ? `${documentUrlPath}/${assetName}` : assetName;
  return `/api/assets?${new URLSearchParams({ path })}`;
}

export function getAssetThumbnailUrl(
  documentPath: string,
  assetName: string,
): string {
  return `${getAssetUrl(documentPath, assetName)}&thumbnail=1`;
}

export function getDocumentIconUrl(
  documentPath: string,
  modifiedAt: string | null,
): string | null {
  if (!modifiedAt) return null;
  return `${getAssetUrl(documentPath, documentIconName)}&v=${encodeURIComponent(modifiedAt)}`;
}

export function parseAssetPath(
  path: string,
): { documentPath: string; assetName: string } | null {
  if (
    !path ||
    path !== path.trim() ||
    path.includes('\\') ||
    path.includes('\0')
  ) {
    return null;
  }

  const segments = path.split('/');
  if (
    segments.some((segment) => !segment || segment === '.' || segment === '..')
  ) {
    return null;
  }

  const assetName = segments.pop();
  if (!assetName) return null;
  return {
    documentPath: urlPathToDocumentPath(segments.join('/')),
    assetName,
  };
}

function decodeMarkdownPath(source: string): string | null {
  const pathEnd = source.search(/[?#]/);
  const encodedPath = pathEnd < 0 ? source : source.slice(0, pathEnd);
  try {
    return decodeURIComponent(encodedPath).normalize('NFC');
  } catch {
    return null;
  }
}

export function getMarkdownAssetName(
  documentPath: string,
  assets: string[],
  source: string,
): string | null {
  if (
    !source ||
    source.startsWith('#') ||
    source.startsWith('//') ||
    /^[a-z][a-z\d+.-]*:/i.test(source)
  ) {
    return null;
  }

  const decodedPath = decodeMarkdownPath(source);
  if (!decodedPath || decodedPath.includes('\\')) return null;

  const normalizedPath = decodedPath.replace(/^\/+/, '').replace(/^\.\//, '');
  const segments = normalizedPath.split('/');
  if (
    segments.some((segment) => !segment || segment === '.' || segment === '..')
  ) {
    return null;
  }

  const documentDirectory = documentPath.endsWith('/index.md')
    ? documentPath.slice(0, -'/index.md'.length)
    : documentPath === 'index.md'
      ? ''
      : null;
  if (documentDirectory === null) return null;

  return (
    assets.find((assetName) => {
      const normalizedAssetName = assetName.normalize('NFC');
      return (
        normalizedPath === normalizedAssetName ||
        (documentDirectory !== '' &&
          normalizedPath === `${documentDirectory}/${normalizedAssetName}`)
      );
    }) ?? null
  );
}

export function resolveMarkdownAssetUrl(
  documentPath: string,
  assets: string[],
  source: string,
): string {
  const assetName = getMarkdownAssetName(documentPath, assets, source);
  return assetName ? getAssetUrl(documentPath, assetName) : source;
}

export function getAssetContentDisposition(
  assetName: string,
  inline: boolean,
): string {
  const fallbackName =
    assetName.replace(/[^\x20-\x7e]|["\\]/g, '_') || 'download';
  const encodedName = encodeURIComponent(assetName).replace(
    /['()*]/g,
    (character) => `%${character.charCodeAt(0).toString(16).toUpperCase()}`,
  );
  return `${inline ? 'inline' : 'attachment'}; filename="${fallbackName}"; filename*=UTF-8''${encodedName}`;
}
