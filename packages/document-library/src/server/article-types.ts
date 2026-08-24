export interface ArticleMetadata {
  path: string;
  title: string;
  icon: string | null;
  summary: string | null;
  lineCount: number;
  attachmentCount: number;
}

export interface ArticleListItem extends ArticleMetadata {
  childCount: number;
}

export interface ArticleAttachment {
  name: string;
  size: number;
  modifiedAt: string;
}

export interface ReadArticleAttachmentInput {
  path: string;
  name: string;
  offset?: number;
  maxBytes?: number;
}

export interface ReadArticleAttachmentResult {
  attachment: ArticleAttachment;
  data: ArrayBuffer;
  offset: number;
  nextOffset: number | null;
}

export interface WriteArticleAttachmentInput {
  path: string;
  name: string;
  data: Uint8Array;
  overwrite?: boolean;
}

export interface DeleteArticleAttachmentInput {
  path: string;
  name: string;
}

export interface DeleteArticleAttachmentResult {
  path: string;
  name: string;
  deleted: true;
}

export interface ReadArticleLinesInput {
  path: string;
  offset?: number;
  limit?: number;
}

export interface ReadArticleLinesResult {
  article: ArticleMetadata;
  revision: string;
  offset: number;
  endLine: number;
  totalLines: number;
  content: string;
  nextOffset: number | null;
}

export interface WriteArticleContentInput {
  path: string;
  expectedRevision: string;
  content: string;
}

export interface WriteArticleContentResult {
  article: ArticleMetadata;
  revision: string;
  changed: boolean;
  previousTotalLines: number;
  totalLines: number;
}

export interface EditArticleContentInput {
  path: string;
  expectedRevision: string;
  oldString: string;
  newString: string;
  replaceAll?: boolean;
}

export interface EditArticleContentResult {
  article: ArticleMetadata;
  revision: string;
  replacements: number;
  totalLines: number;
}

export interface ArticleMetadataPatch {
  title?: string;
  icon?: string | null;
  summary?: string | null;
}

export interface CreateArticleInput extends ArticleMetadataPatch {
  path: string;
  content?: string;
}

export interface UpdateArticleInput {
  path: string;
  expectedRevision: string;
  metadata: ArticleMetadataPatch;
}

export interface MoveArticleInput {
  path: string;
  expectedRevision: string;
  newPath: string;
}

export interface MoveArticleResult {
  previousPath: string;
  path: string;
  movedArticles: number;
}

export interface DeleteArticleInput {
  path: string;
  recursive?: boolean;
}

export interface DeleteArticleResult {
  path: string;
  recursive: boolean;
  deletedArticles: number;
}
