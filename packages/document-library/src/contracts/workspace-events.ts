import type { LibraryNavigationDocument } from './library-api.js';
import type { LibraryRepositoryChange } from './repository.js';
import type { LibraryStatus } from './library-types.js';

export interface WorkspaceEventSnapshot {
  navigation: {
    directories: string[];
    documents: LibraryNavigationDocument[];
  };
  repositoryChanges: LibraryRepositoryChange[];
  repositoryError: string | null;
  repositoryRevision: string | null;
  status: LibraryStatus;
}
