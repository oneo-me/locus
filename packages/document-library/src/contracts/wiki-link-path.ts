/**
 * 浏览器与服务端共用的 wiki 链接路径解析（不依赖 Node API）。
 */

import { stripLibraryReferenceScheme } from './library-reference';

function normalizeArticlePath(path: string): string {
  const trimmed = stripLibraryReferenceScheme(path.trim());
  if (
    trimmed.startsWith('/') ||
    trimmed.includes('\\') ||
    trimmed.includes('\0')
  ) {
    throw new Error('path 必须是文档库内的安全相对路径。');
  }
  const segments = trimmed.split('/').filter(Boolean);
  if (segments.some((segment) => segment === '.' || segment === '..')) {
    throw new Error('path 不能包含 "." 或 ".." 路径段。');
  }
  return segments.join('/');
}

/** 解析 `[[...]]` 目标为库相对文章路径；非法目标返回 null。 */
export function parseWikiLinkArticlePath(raw: string): string | null {
  const trimmed = raw.trim();
  if (!trimmed || trimmed.includes('|') || trimmed.includes('\n')) {
    return null;
  }

  const withoutIndex = stripLibraryReferenceScheme(trimmed)
    .replace(/\/index\.md$/i, '')
    .replace(/^index\.md$/i, '');

  try {
    return normalizeArticlePath(withoutIndex);
  } catch {
    return null;
  }
}
