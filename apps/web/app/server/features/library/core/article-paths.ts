import { dirname } from 'node:path';
import { documentIndexName, normalizeRelativePath } from './paths.js';

export function normalizeArticlePath(path: string): string {
  return normalizeRelativePath(path);
}

export function articlePathToDocumentPath(path: string): string {
  const normalized = normalizeArticlePath(path);
  return normalized ? `${normalized}/${documentIndexName}` : documentIndexName;
}

export function documentPathToArticlePath(path: string): string {
  const normalized = normalizeRelativePath(path);
  if (normalized === documentIndexName) return '';
  const suffix = `/${documentIndexName}`;
  if (!normalized.endsWith(suffix)) {
    throw new Error('内部文档路径无法映射为文章路径。');
  }
  return normalized.slice(0, -suffix.length);
}

export function articleParentPath(path: string): string {
  const normalized = normalizeArticlePath(path);
  if (!normalized) return '';
  const parent = dirname(normalized);
  return parent === '.' ? '' : parent;
}
