import type { LibraryNavigationDocument } from '@locus/document-library/contracts/library-api';
import type { DocumentLibrary } from '@locus/document-library/server/document-library';
import type { DocumentSummary } from '@locus/document-library/contracts/library-types';

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

export function readLibraryNavigation(library: DocumentLibrary): {
  directories: string[];
  documents: LibraryNavigationDocument[];
} {
  return {
    directories: library.listAllDirectories(),
    documents: library.listAllDocuments().map(toLibraryNavigationDocument),
  };
}
