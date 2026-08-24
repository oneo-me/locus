import type { DocumentLibrary } from '@locus/document-library/server/document-library';
import type {
  LibraryRepositoryChange,
  LibraryRepositoryChanges,
} from '@locus/document-library/contracts/repository';

export interface LibraryRepositoryView {
  revision: string | null;
  changes: LibraryRepositoryChange[];
  error: string | null;
}

function gitRepositoryDisabled(): boolean {
  const value = process.env.ASTERWEFT_GIT_DISABLED;
  return value === '1' || value === 'true';
}

const loggedRepositoryErrors = new WeakMap<DocumentLibrary, string>();
export function readLibraryRepositoryView(
  library: DocumentLibrary,
): LibraryRepositoryView {
  if (gitRepositoryDisabled()) {
    return { revision: null, changes: [], error: null };
  }
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
      console.error('无法读取文档库 Git 状态。', error);
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
