import type { LibraryNavigationDocument } from '@locus/document-library/contracts/library-api';
import type { DocumentLibrary } from '@locus/document-library/server/document-library';
import type { DocumentSummary } from '@locus/document-library/contracts/library-types';
import {
  archivedDocumentPaths,
  isDocumentArchived,
} from '@locus/document-library/contracts/document-archive';

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
  const documents = library.listAllDocuments();
  const archivedPaths = archivedDocumentPaths(documents);
  return {
    directories: library
      .listAllDirectories()
      .filter(
        (directory) =>
          !isDocumentArchived(archivedPaths, `${directory}/index.md`),
      ),
    documents: documents
      .filter((document) => !isDocumentArchived(archivedPaths, document.path))
      .map(toLibraryNavigationDocument),
  };
}
