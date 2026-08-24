import type {
  LibraryNavigationDocument,
  LibraryNavigationSearchResult,
} from '../../contracts/library-api';
import type { DocumentGraph } from '../library/core/document-graph';
import type { LibraryRepositoryChange } from '../library/core/git-repository';
import type { LibraryStatus } from '../library/core/types';

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
