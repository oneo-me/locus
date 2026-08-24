import { mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import type { DocumentLibrary } from '~/server/features/library/core/document-library';
import type { DocumentAsset } from '~/server/features/library/core/types';
import { afterEach, describe, expect, test, vi } from 'vitest';
import { createRouteAssetResponse } from './route.server';

const attachment: DocumentAsset = {
  data: new TextEncoder().encode('attachment').buffer,
  name: '报告.pdf',
  size: 10,
};
const cacheDirs: string[] = [];

function temporaryCacheDir(): string {
  const path = mkdtempSync(join(tmpdir(), 'asterweft-route-thumbnails-'));
  cacheDirs.push(path);
  return path;
}

afterEach(() => {
  for (const path of cacheDirs.splice(0)) {
    rmSync(path, { recursive: true, force: true });
  }
});

function libraryStub({
  articleExists = false,
  asset = attachment,
  assetError,
}: {
  articleExists?: boolean;
  asset?: DocumentAsset;
  assetError?: Error;
} = {}): DocumentLibrary {
  return {
    getDocument: vi.fn(() => {
      if (articleExists) return { path: 'guide/报告.pdf/index.md' };
      throw new Error('文章不存在。');
    }),
    readAsset: vi.fn(() => {
      if (assetError) throw assetError;
      return asset;
    }),
    readDocumentIcon: vi.fn(() => asset),
  } as unknown as DocumentLibrary;
}

describe('createRouteAssetResponse', () => {
  test('does not treat an existing article path as an attachment', async () => {
    const library = libraryStub({ articleExists: true });

    await expect(
      createRouteAssetResponse(
        new Request('http://localhost/guide/report'),
        'guide/report',
        library,
      ),
    ).resolves.toBeNull();
    expect(library.readAsset).not.toHaveBeenCalled();
  });

  test('serves the final segment as an attachment of its parent article', async () => {
    const library = libraryStub();
    const response = await createRouteAssetResponse(
      new Request('http://localhost/guide/%E6%8A%A5%E5%91%8A.pdf'),
      'guide/报告.pdf',
      library,
    );

    expect(response?.status).toBe(200);
    expect(response?.headers.get('Content-Disposition')).toContain('inline;');
    expect(library.readAsset).toHaveBeenCalledWith(
      'guide/index.md',
      '报告.pdf',
    );
  });

  test('serves the protected icon.png through the attachment endpoint', async () => {
    const icon = {
      ...attachment,
      name: 'icon.png',
    };
    const library = libraryStub({ asset: icon });
    const response = await createRouteAssetResponse(
      new Request('http://localhost/guide/icon.png'),
      'guide/icon.png',
      library,
    );

    expect(response?.status).toBe(200);
    expect(response?.headers.get('Content-Type')).toBe('image/png');
    expect(library.readDocumentIcon).toHaveBeenCalledWith('guide/index.md');
    expect(library.readAsset).not.toHaveBeenCalled();
  });

  test('falls through when neither an article nor an attachment exists', async () => {
    const library = libraryStub({
      assetError: new Error('附件不存在。'),
    });

    await expect(
      createRouteAssetResponse(
        new Request('http://localhost/guide/missing.bin'),
        'guide/missing.bin',
        library,
      ),
    ).resolves.toBeNull();
  });

  test('generates and serves cached WebP thumbnails for image attachments', async () => {
    const data = new TextEncoder().encode(
      '<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="600"><rect width="100%" height="100%" fill="navy"/></svg>',
    );
    const library = libraryStub({
      asset: { data: data.buffer, name: 'diagram.svg', size: data.byteLength },
    });
    const cacheDir = temporaryCacheDir();
    const response = await createRouteAssetResponse(
      new Request(
        'http://localhost/api/assets?path=guide%2Fdiagram.svg&thumbnail=1',
      ),
      'guide/diagram.svg',
      library,
      cacheDir,
    );

    expect(response?.status).toBe(200);
    expect(response?.headers.get('Content-Type')).toBe('image/webp');
    expect(response?.headers.get('Content-Disposition')).toContain('inline;');
    expect(readdirSync(cacheDir)).toHaveLength(1);
  });

  test('rejects thumbnail requests for non-image attachments', async () => {
    const response = await createRouteAssetResponse(
      new Request(
        'http://localhost/api/assets?path=guide%2Freport.pdf&thumbnail=1',
      ),
      'guide/report.pdf',
      libraryStub(),
      temporaryCacheDir(),
    );

    expect(response?.status).toBe(415);
    await expect(response?.text()).resolves.toBe('该附件无法生成图片缩略图。');
  });
});
