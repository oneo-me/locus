import type {
  LibraryNavigationDocument,
  LibraryNavigationSearchResult,
} from '~/server/contracts/library-api';
import type { DocumentLibrary } from './core/document-library';
import type { DocumentSummary } from './core/types';

export function toLibraryNavigationDocument(
  document: DocumentSummary,
): LibraryNavigationDocument {
  return {
    attachmentCount: document.assets.length,
    directory: document.directory,
    icon: document.icon,
    iconImageModifiedAt: document.iconImageModifiedAt,
    path: document.path,
    title: document.title,
  };
}

export function readLibraryNavigation(
  library: DocumentLibrary,
  query?: string,
): {
  directories: string[];
  documents: LibraryNavigationDocument[];
  search: LibraryNavigationSearchResult | null;
} {
  const search = query ? library.searchDocuments(query, 100) : null;
  return {
    directories: library.listAllDirectories(),
    documents: library.listAllDocuments().map(toLibraryNavigationDocument),
    search: search
      ? {
          query: search.query,
          total: search.total,
          results: search.results.map(toLibraryNavigationDocument),
        }
      : null,
  };
}
