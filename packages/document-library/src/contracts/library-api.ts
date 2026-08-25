import type { DocumentGraph } from './document-graph.js';
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

export interface LibraryWorkspaceView {
  directories: string[];
  documents: LibraryNavigationDocument[];
  libraryState: LibraryState;
  repositoryChanges: LibraryRepositoryChange[];
  repositoryError: string | null;
  repositoryRevision: string | null;
  status: LibraryStatus;
}

export interface LibraryGraph {
  graph: DocumentGraph;
  status: LibraryStatus;
}

export interface LibraryArchiveView {
  documents: LibraryNavigationDocument[];
}

export interface UpdateDocumentIconImageInput {
  path: string;
  dataBase64: string | null;
}

export interface OpenDocumentInput {
  path: string;
  target: 'directory' | 'vscode';
}

export interface ReorderDocumentsInput {
  items: Array<{ path: string; order: number }>;
}

export interface SetDocumentGraphParticipationInput {
  path: string;
  participates: boolean;
}

export interface SetDocumentArchivedInput {
  path: string;
  archived: boolean;
}
