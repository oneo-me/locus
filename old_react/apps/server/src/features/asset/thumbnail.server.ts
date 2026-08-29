import { createHash, randomUUID } from 'node:crypto';
import {
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { join } from 'node:path';
import { resolveDataDirectory } from '~/data-directory.server';
import sharp from 'sharp';
import type {
  DocumentAsset,
  DocumentImageAsset,
} from '@locus/document-library/contracts/library-types';

export const assetThumbnailMaxSize = 640;
const assetThumbnailQuality = 75;
const assetThumbnailVersion = 1;
interface ThumbnailData {
  data: Uint8Array;
  height: number;
  width: number;
}

const pendingThumbnails = new Map<string, Promise<ThumbnailData>>();

export function ensureAssetThumbnailCacheDir(): string {
  const directory = join(resolveDataDirectory(), 'thumbnails');
  mkdirSync(directory, { recursive: true });
  return directory;
}

export function assetThumbnailCacheKey(asset: DocumentAsset): string {
  return createHash('sha256')
    .update(`locus-thumbnail-v${assetThumbnailVersion}\0`)
    .update(new Uint8Array(asset.data))
    .digest('hex');
}

export function resolveAssetThumbnailPath(
  asset: DocumentAsset,
  cacheDir: string,
): string {
  return join(cacheDir, `${assetThumbnailCacheKey(asset)}.webp`);
}

async function generateThumbnail(
  asset: DocumentAsset,
  cachePath: string,
): Promise<ThumbnailData> {
  const { data, info } = await sharp(new Uint8Array(asset.data))
    .rotate()
    .resize(assetThumbnailMaxSize, assetThumbnailMaxSize, {
      fit: 'inside',
      withoutEnlargement: true,
    })
    .webp({ quality: assetThumbnailQuality })
    .toBuffer({ resolveWithObject: true });
  const temporaryPath = `${cachePath}.${process.pid}.${randomUUID()}.tmp`;
  try {
    writeFileSync(temporaryPath, data, { flag: 'wx' });
    renameSync(temporaryPath, cachePath);
  } finally {
    rmSync(temporaryPath, { force: true });
  }
  return { data, height: info.height, width: info.width };
}

async function readCachedThumbnail(cachePath: string): Promise<ThumbnailData> {
  const data = Uint8Array.from(readFileSync(cachePath));
  const metadata = await sharp(data).metadata();
  if (!metadata.height || !metadata.width) {
    throw new Error('无法读取缩略图尺寸。');
  }
  return { data, height: metadata.height, width: metadata.width };
}

export async function readAssetThumbnail(
  asset: DocumentAsset,
  cacheDir: string,
): Promise<DocumentImageAsset> {
  const cachePath = resolveAssetThumbnailPath(asset, cacheDir);
  let pending = pendingThumbnails.get(cachePath);
  if (!pending) {
    pending = existsSync(cachePath)
      ? readCachedThumbnail(cachePath)
      : generateThumbnail(asset, cachePath);
    pendingThumbnails.set(cachePath, pending);
  }

  try {
    const thumbnail = await pending;
    const arrayBuffer = Uint8Array.from(thumbnail.data).buffer;
    return {
      data: arrayBuffer,
      height: thumbnail.height,
      name: 'thumbnail.webp',
      size: arrayBuffer.byteLength,
      width: thumbnail.width,
    };
  } finally {
    pendingThumbnails.delete(cachePath);
  }
}
