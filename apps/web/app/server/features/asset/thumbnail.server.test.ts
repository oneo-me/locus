import { mkdtempSync, readdirSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import sharp from 'sharp';
import { afterEach, describe, expect, test } from 'vitest';
import type { DocumentAsset } from '~/server/features/library/core/types';
import {
  assetThumbnailCacheKey,
  assetThumbnailMaxSize,
  readAssetThumbnail,
  resolveAssetThumbnailPath,
} from './thumbnail.server';

const cacheDirs: string[] = [];

function temporaryCacheDir(): string {
  const path = mkdtempSync(join(tmpdir(), 'asterweft-thumbnails-'));
  cacheDirs.push(path);
  return path;
}

function svgAsset(width: number, height: number, fill = 'navy'): DocumentAsset {
  const data = new TextEncoder().encode(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"><rect width="100%" height="100%" fill="${fill}"/></svg>`,
  );
  return { data: data.buffer, name: 'diagram.svg', size: data.byteLength };
}

afterEach(() => {
  for (const path of cacheDirs.splice(0)) {
    rmSync(path, { recursive: true, force: true });
  }
});

describe('asset thumbnails', () => {
  test('generates a bounded WebP without enlarging small images', async () => {
    const cacheDir = temporaryCacheDir();
    const large = await readAssetThumbnail(svgAsset(1280, 640), cacheDir);
    const small = await readAssetThumbnail(svgAsset(120, 60), cacheDir);
    const largeMetadata = await sharp(new Uint8Array(large.data)).metadata();
    const smallMetadata = await sharp(new Uint8Array(small.data)).metadata();

    expect(large.name).toBe('thumbnail.webp');
    expect(large.width).toBe(assetThumbnailMaxSize);
    expect(large.height).toBe(assetThumbnailMaxSize / 2);
    expect(largeMetadata.format).toBe('webp');
    expect(largeMetadata.width).toBe(assetThumbnailMaxSize);
    expect(largeMetadata.height).toBe(assetThumbnailMaxSize / 2);
    expect(smallMetadata.width).toBe(120);
    expect(smallMetadata.height).toBe(60);
    expect(small).toMatchObject({ width: 120, height: 60 });
  });

  test('reuses cached bytes and changes the path when source content changes', async () => {
    const cacheDir = temporaryCacheDir();
    const firstAsset = svgAsset(800, 400, 'navy');
    const changedAsset = svgAsset(800, 400, 'orange');

    await readAssetThumbnail(firstAsset, cacheDir);
    const cachePath = resolveAssetThumbnailPath(firstAsset, cacheDir);
    const firstModifiedAt = statSync(cachePath, { bigint: true }).mtimeNs;
    await readAssetThumbnail(firstAsset, cacheDir);

    expect(statSync(cachePath, { bigint: true }).mtimeNs).toBe(firstModifiedAt);
    expect(assetThumbnailCacheKey(changedAsset)).not.toBe(
      assetThumbnailCacheKey(firstAsset),
    );
    await readAssetThumbnail(changedAsset, cacheDir);
    expect(readdirSync(cacheDir)).toHaveLength(2);
  });
});
