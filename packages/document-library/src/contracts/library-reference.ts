/**
 * 文章引用的统一 URI 形式：`asterweft://<库内文章路径>`。
 * 浏览器与服务端共用，作为引用格式解析的唯一事实来源。
 */

export const libraryReferenceScheme = 'asterweft://';

export function stripLibraryReferenceScheme(path: string): string {
  if (!path.toLowerCase().startsWith(libraryReferenceScheme)) return path;
  return path.slice(libraryReferenceScheme.length);
}
