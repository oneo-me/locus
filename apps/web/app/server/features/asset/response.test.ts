import type { DocumentAsset } from '~/server/features/library/core/types';
import { describe, expect, test } from 'vitest';
import {
  createAssetResponse,
  parseAssetByteRange,
  resolveHtmlAssetUrl,
  rewriteHtmlAssetUrls,
} from './response';

const bytes = Uint8Array.from({ length: 20 }, (_, index) => index);
const asset: DocumentAsset = {
  name: '示意图.jpg',
  data: bytes.buffer,
  size: bytes.byteLength,
};

describe('parseAssetByteRange', () => {
  test('parses bounded, open-ended, and suffix ranges', () => {
    expect(parseAssetByteRange('bytes=2-5', 20)).toEqual({
      kind: 'partial',
      start: 2,
      end: 5,
    });
    expect(parseAssetByteRange('bytes=15-', 20)).toEqual({
      kind: 'partial',
      start: 15,
      end: 19,
    });
    expect(parseAssetByteRange('bytes=-4', 20)).toEqual({
      kind: 'partial',
      start: 16,
      end: 19,
    });
  });

  test('rejects ranges outside the asset', () => {
    expect(parseAssetByteRange('bytes=20-', 20)).toEqual({
      kind: 'unsatisfiable',
    });
    expect(parseAssetByteRange('bytes=8-3', 20)).toEqual({
      kind: 'unsatisfiable',
    });
  });
});

describe('createAssetResponse', () => {
  test('advertises byte ranges on complete downloads', async () => {
    const response = createAssetResponse(
      new Request('http://localhost/guide/diagram.png'),
      asset,
      false,
    );

    expect(response.status).toBe(200);
    expect(response.statusText).toBe('OK');
    expect(response.headers.get('Accept-Ranges')).toBe('bytes');
    expect(response.headers.get('Content-Length')).toBe('20');
    expect(response.headers.get('Content-Disposition')).toContain(
      'filename="___.jpg"',
    );
    expect(new Uint8Array(await response.arrayBuffer())).toEqual(bytes);
  });

  test('returns a partial response for download segments', async () => {
    const response = createAssetResponse(
      new Request('http://localhost/guide/diagram.png', {
        headers: { Range: 'bytes=5-9' },
      }),
      asset,
      false,
    );

    expect(response.status).toBe(206);
    expect(response.statusText).toBe('Partial Content');
    expect(response.headers.get('Content-Range')).toBe('bytes 5-9/20');
    expect(response.headers.get('Content-Length')).toBe('5');
    expect(new Uint8Array(await response.arrayBuffer())).toEqual(
      bytes.slice(5, 10),
    );
  });

  test('returns range headers without a body for HEAD requests', async () => {
    const response = createAssetResponse(
      new Request('http://localhost/guide/diagram.png', {
        method: 'HEAD',
        headers: { Range: 'bytes=5-' },
      }),
      asset,
      false,
    );

    expect(response.status).toBe(206);
    expect(response.headers.get('Content-Range')).toBe('bytes 5-19/20');
    expect(response.headers.get('Content-Length')).toBe('15');
    expect((await response.arrayBuffer()).byteLength).toBe(0);
  });

  test('returns 416 for an unsatisfiable range', () => {
    const response = createAssetResponse(
      new Request('http://localhost/guide/diagram.png', {
        headers: { Range: 'bytes=20-' },
      }),
      asset,
      false,
    );

    expect(response.status).toBe(416);
    expect(response.headers.get('Content-Range')).toBe('bytes */20');
  });

  test('serves inline HTML in a restricted browser sandbox', async () => {
    const data = new TextEncoder().encode(
      '<!doctype html><style>body { color: navy }</style><h1>报告</h1><script>window.opener.document.body.remove()</script>',
    );
    const response = createAssetResponse(
      new Request('http://localhost/guide/report.html'),
      {
        data: data.buffer,
        name: 'report.html',
        size: data.byteLength,
      },
      true,
      'guide/index.md',
    );

    expect(response.headers.get('Content-Disposition')).toContain('inline;');
    expect(response.headers.get('Content-Type')).toBe(
      'text/html; charset=utf-8',
    );
    expect(response.headers.get('Content-Security-Policy')).toBe(
      "sandbox allow-same-origin; default-src 'none'; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; font-src 'self' data:; media-src 'self' data: blob:",
    );
    expect(response.headers.get('Content-Security-Policy')).not.toContain(
      'allow-scripts',
    );
    const html = await response.text();
    expect(html).toContain(
      '<!doctype html><style>body { color: navy }</style><h1>报告</h1>',
    );
    expect(html).toContain('<h1>报告</h1>');
  });

  test('rewrites sibling HTML resources to the existing attachment route', () => {
    expect(
      rewriteHtmlAssetUrls(
        '<img src="杜佳颖头像.png"><a href="./简历.pdf">简历</a>',
        'others/du-jiaying-resume/index.md',
      ),
    ).toBe(
      '<img src="/api/assets?path=others%2Fdu-jiaying-resume%2F%E6%9D%9C%E4%BD%B3%E9%A2%96%E5%A4%B4%E5%83%8F.png"><a href="/api/assets?path=others%2Fdu-jiaying-resume%2F%E7%AE%80%E5%8E%86.pdf">简历</a>',
    );
  });

  test('resolves root-looking sibling names but leaves external URLs unchanged', () => {
    expect(
      resolveHtmlAssetUrl(
        'others/du-jiaying-resume/index.md',
        '/杜佳颖头像.png',
      ),
    ).toBe(
      '/api/assets?path=others%2Fdu-jiaying-resume%2F%E6%9D%9C%E4%BD%B3%E9%A2%96%E5%A4%B4%E5%83%8F.png',
    );
    expect(
      resolveHtmlAssetUrl(
        'others/du-jiaying-resume/index.md',
        'https://example.com/avatar.png',
      ),
    ).toBe('https://example.com/avatar.png');
  });
});
