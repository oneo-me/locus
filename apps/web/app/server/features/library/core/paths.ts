import { lstatSync, realpathSync } from 'node:fs';
import {
  basename,
  dirname,
  isAbsolute,
  relative,
  resolve,
  sep,
} from 'node:path';
import { stripLibraryReferenceScheme } from '~/contracts/library-reference';
import { isDocumentIconName } from './document-icon.js';

export const documentIndexName = 'index.md';

export function normalizeRelativePath(path: string): string {
  const trimmed = stripLibraryReferenceScheme(path.trim());
  if (isAbsolute(trimmed) || trimmed.includes('\\') || trimmed.includes('\0')) {
    throw new Error('path 必须是文档库内的安全相对路径。');
  }
  const segments = trimmed.split('/').filter(Boolean);
  if (segments.some((segment) => segment === '.' || segment === '..')) {
    throw new Error('path 不能包含 "." 或 ".." 路径段。');
  }
  return segments.join('/');
}

export function isDocumentIndexPath(path: string): boolean {
  return basename(path) === documentIndexName;
}

export function isInside(parent: string, candidate: string): boolean {
  const pathFromParent = relative(parent, candidate);
  return (
    pathFromParent === '' ||
    (!pathFromParent.startsWith(`..${sep}`) &&
      pathFromParent !== '..' &&
      !isAbsolute(pathFromParent))
  );
}

export function resolveExistingDocument(
  rootPath: string,
  path: string,
): string {
  const normalized = normalizeRelativePath(path);
  if (!normalized || !isDocumentIndexPath(normalized)) {
    throw new Error('path 必须指向文档目录中的 index.md。');
  }
  const unresolved = resolve(rootPath, normalized);
  try {
    const entry = lstatSync(unresolved);
    const target = realpathSync(unresolved);
    if (
      entry.isSymbolicLink() ||
      !entry.isFile() ||
      !isInside(rootPath, target)
    ) {
      throw new Error();
    }
    return target;
  } catch {
    throw new Error(`文档不存在或无法访问：${normalized}`);
  }
}

export function resolveNewDocument(rootPath: string, path: string): string {
  const normalized = normalizeRelativePath(path);
  if (!normalized || !isDocumentIndexPath(normalized)) {
    throw new Error('path 必须指向新文档目录中的 index.md。');
  }
  const target = resolve(rootPath, normalized);
  if (!isInside(rootPath, target)) {
    throw new Error('path 超出文档库范围。');
  }
  return target;
}

export function resolveExistingAsset(
  rootPath: string,
  documentPath: string,
  assetName: string,
): string {
  const name = normalizeAssetName(assetName);
  const documentTarget = resolveExistingDocument(rootPath, documentPath);
  const documentDirectory = dirname(documentTarget);
  const unresolved = resolve(documentDirectory, name);
  try {
    const directoryEntry = lstatSync(documentDirectory);
    const realDirectory = realpathSync(documentDirectory);
    const assetEntry = lstatSync(unresolved);
    const target = realpathSync(unresolved);
    if (
      directoryEntry.isSymbolicLink() ||
      !directoryEntry.isDirectory() ||
      !isInside(rootPath, realDirectory) ||
      !isInside(documentDirectory, unresolved) ||
      assetEntry.isSymbolicLink() ||
      !assetEntry.isFile() ||
      !isInside(realDirectory, target)
    ) {
      throw new Error();
    }
    return target;
  } catch {}

  throw new Error(`附件不存在或无法访问：${name}`);
}

export function resolveNewAsset(
  rootPath: string,
  documentPath: string,
  assetName: string,
): string {
  const name = normalizeAssetName(assetName);
  if (
    name.startsWith('.') ||
    name.toLowerCase() === documentIndexName ||
    isDocumentIconName(name)
  ) {
    throw new Error('附件名不能是隐藏文件、文章正文文件或受保护的 icon.png。');
  }

  const documentTarget = resolveExistingDocument(rootPath, documentPath);
  const documentDirectory = dirname(documentTarget);
  const unresolved = resolve(documentDirectory, name);
  const directoryEntry = lstatSync(documentDirectory);
  const realDirectory = realpathSync(documentDirectory);
  if (
    directoryEntry.isSymbolicLink() ||
    !directoryEntry.isDirectory() ||
    !isInside(rootPath, realDirectory) ||
    !isInside(realDirectory, unresolved)
  ) {
    throw new Error(`附件存储位置无法安全访问：${name}`);
  }
  return unresolved;
}

function normalizeAssetName(assetName: string): string {
  const name = assetName.trim();
  if (
    !name ||
    name !== assetName ||
    basename(name) !== name ||
    name.includes('\\') ||
    name.includes('\0') ||
    new TextEncoder().encode(name).byteLength > 255
  ) {
    throw new Error('assetName 必须是安全的附件文件名。');
  }
  return name;
}
