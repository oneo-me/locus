import type { LibraryRepositoryChange } from './repository.js';
import type {
  DocumentContent,
  DocumentSummary,
  LibraryStatus,
} from './library-types.js';
import type { LibraryState } from './library-state.js';

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

export interface LibrarySelectionSourceView extends Omit<
  LibrarySelectionView,
  'selected'
> {
  assetImageSizes: Record<string, { height: number; width: number }>;
  selected: DocumentContent | null;
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
