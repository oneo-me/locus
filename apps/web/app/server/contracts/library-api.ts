import type { DocumentGraph } from '../features/library/core/document-graph';
import type { LibraryRepositoryChange } from '../features/library/core/git-repository';
import type {
  DocumentContent,
  DocumentSummary,
  LibraryStatus,
} from '../features/library/core/types';
import type { LibraryState } from '../features/library/state';

export interface SerializedMarkdownTree {
  frontmatter: Record<string, unknown>;
  meta: Record<string, unknown>;
  nodes: unknown[];
}

export interface LibraryWorkspaceViewInput {
  query?: string;
}

export interface LibrarySelectionViewInput {
  doc: string;
}

export interface LibrarySelectionView {
  indexedAt: string | null;
  selected:
    | (Omit<DocumentContent, 'content'> & {
        content: SerializedMarkdownTree;
      })
    | null;
  selectedPath: string | null;
  selectionError: string | null;
}

export type LibraryNavigationDocument = Pick<
  DocumentSummary,
  'directory' | 'icon' | 'iconImageModifiedAt' | 'path' | 'title'
> & {
  attachmentCount: number;
};

export interface LibraryNavigationSearchResult {
  query: string;
  total: number;
  results: LibraryNavigationDocument[];
}

export interface LibraryWorkspaceView {
  directories: string[];
  documents: LibraryNavigationDocument[];
  libraryState: LibraryState;
  repositoryChanges: LibraryRepositoryChange[];
  repositoryError: string | null;
  repositoryRevision: string | null;
  search: LibraryNavigationSearchResult | null;
  status: LibraryStatus;
}

export interface LibraryGraph {
  graph: DocumentGraph;
  status: LibraryStatus;
}

export interface UpdateDocumentMetadataInput {
  path: string;
  icon?: string | null;
}

export interface UpdateDocumentIconImageInput {
  path: string;
  dataBase64: string | null;
}

export interface ReorderDocumentsInput {
  items: Array<{ path: string; order: number }>;
}

export interface SetDocumentGraphParticipationInput {
  path: string;
  participates: boolean;
}
