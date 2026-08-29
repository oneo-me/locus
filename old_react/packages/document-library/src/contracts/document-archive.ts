import type { DocumentSummary } from './library-types.js';

export function archivedDocumentPaths(
  documents: readonly DocumentSummary[],
): string[] {
  return documents
    .filter((document) => document.archived)
    .map((document) => document.path);
}

export function isDocumentArchived(
  archivedPaths: readonly string[],
  path: string,
): boolean {
  return archivedPaths.some((archivedPath) => {
    if (archivedPath === path) return true;
    const directory = archivedPath.replace(/(^|\/)index\.md$/, '');
    return directory.length === 0 || path.startsWith(`${directory}/`);
  });
}
