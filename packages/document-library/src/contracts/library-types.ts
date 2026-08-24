export interface DocumentMetadata {
  title: string;
  icon: string | null;
  summary: string | null;
}

export type DocumentMetadataKey = keyof DocumentMetadata;

export const documentMetadataKeys: DocumentMetadataKey[] = [
  'title',
  'icon',
  'summary',
];

export interface DocumentSummary extends DocumentMetadata {
  path: string;
  directory: string;
  assets: string[];
  iconImageModifiedAt: string | null;
  metadataKeys: DocumentMetadataKey[];
  modifiedAt: string;
  size: number;
  wordCount: number;
  issue: string | null;
}

export interface DocumentContent extends DocumentSummary {
  content: string;
}

export interface DocumentAsset {
  name: string;
  data: ArrayBuffer;
  size: number;
}

export interface ImageSize {
  height: number;
  width: number;
}

export interface DocumentImageAsset extends DocumentAsset, ImageSize {}

export interface DocumentExcerpt extends DocumentSummary {
  excerpt: string;
}

export interface ListDocumentsResult {
  directory: string;
  directories: string[];
  documents: DocumentSummary[];
}

export interface SearchDocumentsResult {
  query: string;
  total: number;
  results: DocumentExcerpt[];
}

export interface LibraryStatus {
  rootPath: string;
  documentCount: number;
  watching: boolean;
  refreshing: boolean;
  indexedAt: string | null;
}

export interface CreateDocumentInput {
  path: string;
  title?: string;
  icon?: string | null;
  summary?: string | null;
  content?: string;
}

export interface UpdateDocumentInput {
  path: string;
  title?: string;
  icon?: string | null;
  summary?: string | null;
  content?: string;
}
