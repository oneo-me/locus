import type { DocumentLibrary } from './core/document-library';
import { documentIndexName } from './core/paths';
import type { DocumentContent } from './core/types';
import { documentPathToUrlPath } from './route';

export interface LibrarySelection {
  selected: DocumentContent | null;
  selectedPath: string | null;
  selectionError: string | null;
}

export function resolveLibrarySelection(
  library: DocumentLibrary,
  requestedPath: string | undefined,
): LibrarySelection {
  if (requestedPath === documentIndexName) {
    return {
      selected: null,
      selectedPath: null,
      selectionError: null,
    };
  }

  const directories = library.listAllDirectories();
  let selected: DocumentContent | null = null;
  let selectedPath: string | null = null;
  let selectionError: string | null = null;

  if (requestedPath) {
    try {
      selected = library.getDocument(requestedPath);
      selectedPath = selected.path;
    } catch (error) {
      const requestedDirectory = documentPathToUrlPath(requestedPath);
      if (directories.includes(requestedDirectory)) {
        selectedPath = requestedDirectory;
      } else {
        selectionError = (error as Error).message;
      }
    }
  }

  return { selected, selectedPath, selectionError };
}
