import type { DocumentAsset } from '~/server/features/library/core/types';
import {
  getAssetContentDisposition,
  getAssetContentType,
  getAssetUrl,
} from './protocol';

export type AssetByteRange =
  | { kind: 'full' }
  | { kind: 'partial'; start: number; end: number }
  | { kind: 'unsatisfiable' };

export function parseAssetByteRange(
  header: string | null,
  size: number,
): AssetByteRange {
  if (!header) return { kind: 'full' };
  const match = header.match(/^bytes=(\d*)-(\d*)$/i);
  if (!match) return { kind: 'full' };

  const startText = match[1] ?? '';
  const endText = match[2] ?? '';
  if (!startText && !endText) return { kind: 'full' };
  if (size <= 0) return { kind: 'unsatisfiable' };

  if (!startText) {
    const suffixLength = Number(endText);
    if (!Number.isSafeInteger(suffixLength) || suffixLength <= 0) {
      return { kind: 'unsatisfiable' };
    }
    return {
      kind: 'partial',
      start: Math.max(0, size - suffixLength),
      end: size - 1,
    };
  }

  const start = Number(startText);
  const requestedEnd = endText ? Number(endText) : size - 1;
  if (
    !Number.isSafeInteger(start) ||
    !Number.isSafeInteger(requestedEnd) ||
    start < 0 ||
    start >= size ||
    requestedEnd < start
  ) {
    return { kind: 'unsatisfiable' };
  }

  return {
    kind: 'partial',
    start,
    end: Math.min(requestedEnd, size - 1),
  };
}

export function createAssetResponse(
  request: Request,
  asset: DocumentAsset,
  inline: boolean,
  documentPath?: string,
): Response {
  const contentType = getAssetContentType(asset.name);
  const data =
    contentType === 'text/html; charset=utf-8' && documentPath
      ? new TextEncoder().encode(
          rewriteHtmlAssetUrls(
            new TextDecoder().decode(asset.data),
            documentPath,
          ),
        )
      : new Uint8Array(asset.data);
  const range = parseAssetByteRange(
    request.headers.get('Range'),
    data.byteLength,
  );
  const headers = new Headers({
    'Accept-Ranges': 'bytes',
    'Cache-Control': 'private, no-store',
    'Content-Disposition': getAssetContentDisposition(asset.name, inline),
    'Content-Security-Policy':
      "sandbox allow-same-origin; default-src 'none'; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; font-src 'self' data:; media-src 'self' data: blob:",
    'Content-Type': contentType,
    'Cross-Origin-Resource-Policy': 'same-origin',
    'X-Content-Type-Options': 'nosniff',
  });

  if (range.kind === 'unsatisfiable') {
    headers.set('Content-Length', '0');
    headers.set('Content-Range', `bytes */${data.byteLength}`);
    return new Response(null, {
      status: 416,
      statusText: 'Range Not Satisfiable',
      headers,
    });
  }

  const partial = range.kind === 'partial';
  const start = partial ? range.start : 0;
  const end = partial ? range.end : data.byteLength - 1;
  const length = Math.max(0, end - start + 1);
  headers.set('Content-Length', String(length));
  if (partial) {
    headers.set('Content-Range', `bytes ${start}-${end}/${data.byteLength}`);
  }

  return new Response(
    request.method === 'HEAD' ? null : data.slice(start, end + 1),
    {
      status: partial ? 206 : 200,
      statusText: partial ? 'Partial Content' : 'OK',
      headers,
    },
  );
}

export function resolveHtmlAssetUrl(
  documentPath: string,
  source: string,
): string {
  if (
    !source ||
    source !== source.trim() ||
    source.startsWith('#') ||
    source.startsWith('//') ||
    /^[a-z][a-z\d+.-]*:/i.test(source)
  ) {
    return source;
  }

  const suffixStart = source.search(/[?#]/);
  const encodedPath = suffixStart < 0 ? source : source.slice(0, suffixStart);
  const fragmentStart = source.indexOf('#');
  const fragment = fragmentStart < 0 ? '' : source.slice(fragmentStart);

  let decodedPath: string;
  try {
    decodedPath = decodeURIComponent(encodedPath).normalize('NFC');
  } catch {
    return source;
  }

  const normalizedPath = decodedPath.replace(/^\/+/, '').replace(/^\.\//, '');
  const segments = normalizedPath.split('/');
  if (
    segments.length !== 1 ||
    segments.some((segment) => !segment || segment === '.' || segment === '..')
  ) {
    return source;
  }

  return `${getAssetUrl(documentPath, segments[0])}${fragment}`;
}

export function rewriteHtmlAssetUrls(
  html: string,
  documentPath: string,
): string {
  return html.replace(
    /(\b(?:src|href|poster)\s*=\s*)(?:(["'])(.*?)\2|([^\s"'=<>`]+))/gi,
    (match, prefix, quote, quotedSource, unquotedSource) => {
      const source = quotedSource ?? unquotedSource;
      const resolved = resolveHtmlAssetUrl(documentPath, source);
      if (resolved === source) return match;

      const escaped = resolved.replaceAll('&', '&amp;');
      return quote
        ? `${prefix}${quote}${escaped}${quote}`
        : `${prefix}${escaped}`;
    },
  );
}
