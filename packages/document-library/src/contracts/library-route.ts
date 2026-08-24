const documentIndexName = 'index.md';
const documentIndexSuffix = `/${documentIndexName}`;

export function documentPathToUrlPath(documentPath: string): string {
  if (documentPath === documentIndexName) return '';
  if (documentPath.endsWith(documentIndexSuffix)) {
    return documentPath.slice(0, -documentIndexSuffix.length);
  }
  return documentPath;
}

export function urlPathToDocumentPath(urlPath?: string): string {
  const normalizedPath =
    urlPath?.replace(/^\/+|\/+$/g, '').normalize('NFC') ?? '';
  return normalizedPath
    ? `${normalizedPath}/${documentIndexName}`
    : documentIndexName;
}

export function pathnameToUrlPath(pathname: string): string {
  const normalizedPath = pathname.replace(/^\/+|\/+$/g, '');
  if (!normalizedPath) return '';

  try {
    return normalizedPath
      .split('/')
      .map((segment) => decodeURIComponent(segment))
      .join('/')
      .normalize('NFC');
  } catch {
    return normalizedPath;
  }
}

export function urlPathToHref(urlPath: string): string {
  const normalizedPath = urlPath.replace(/^\/+|\/+$/g, '');
  if (!normalizedPath) return '/';
  return `/${normalizedPath
    .split('/')
    .map((segment) => encodeURIComponent(segment))
    .join('/')}`;
}
