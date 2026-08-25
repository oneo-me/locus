import type { DocumentLibrary } from '@locus/document-library/server/document-library';
import type {
  LibraryRepositoryChange,
  LibraryRepositoryChanges,
} from '@locus/document-library/contracts/repository';
import { logWarn } from '~/logging';

export interface LibraryRepositoryView {
  revision: string | null;
  changes: LibraryRepositoryChange[];
  error: string | null;
}

const loggedRepositoryErrors = new WeakMap<DocumentLibrary, string>();
export function readLibraryRepositoryView(
  library: DocumentLibrary,
): LibraryRepositoryView {
  try {
    const view = {
      ...library.repositoryChanges(),
      error: null,
    };
    loggedRepositoryErrors.delete(library);
    return view;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (loggedRepositoryErrors.get(library) !== message) {
      logWarn('library.repository.read_failed', error);
      loggedRepositoryErrors.set(library, message);
    }
    const view = {
      revision: null,
      changes: [],
      error: message,
    } satisfies LibraryRepositoryView;
    return view;
  }
}

export function repositoryViewChanged(
  previousRevision: LibraryRepositoryChanges['revision'] | null,
  nextRevision: LibraryRepositoryChanges['revision'] | null,
): boolean {
  return previousRevision !== nextRevision;
}
