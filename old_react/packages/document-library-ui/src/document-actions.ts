import { libraryReferenceScheme } from '@locus/document-library/contracts/library-reference';

export function formatDocumentReference(path: string): string {
  return `${libraryReferenceScheme}${path}`;
}
