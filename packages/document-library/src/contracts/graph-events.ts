import type {
  LibraryNavigationDocument,
  LibraryNavigationSearchResult,
} from './library-api.js';
import type { DocumentGraph } from './document-graph.js';
import type { LibraryRepositoryChange } from './repository.js';
import type { LibraryStatus } from './library-types.js';

export interface GraphEventSnapshot {
  graph?: DocumentGraph;
  navigation: {
    directories: string[];
    documents: LibraryNavigationDocument[];
    search: LibraryNavigationSearchResult | null;
  };
  repositoryChanges: LibraryRepositoryChange[];
  repositoryError: string | null;
  repositoryRevision: string | null;
  status: LibraryStatus;
}
