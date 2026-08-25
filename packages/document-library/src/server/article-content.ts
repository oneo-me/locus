import { createHash } from 'node:crypto';
import { documentPathToArticlePath } from './article-paths.js';
import type { ArticleMetadata } from './article-types.js';
import { documentIconName } from '../contracts/document-icon.js';
import type { DocumentContent } from '../contracts/library-types.js';

export function articleContentLines(content: string): string[] {
  if (!content) return [];
  const lines = content.replaceAll('\r\n', '\n').split('\n');
  if (lines.at(-1) === '') lines.pop();
  return lines;
}

export function articleMetadata(document: DocumentContent): ArticleMetadata {
  return {
    archived: document.archived,
    path: documentPathToArticlePath(document.path),
    title: document.title,
    icon:
      document.iconImageModifiedAt === null ? document.icon : documentIconName,
    summary: document.summary,
    lineCount: articleContentLines(document.content).length,
    attachmentCount: document.assets.length,
  };
}

export function articleRevision(document: DocumentContent): string {
  return createHash('sha256')
    .update(
      JSON.stringify({
        path: document.path,
        archived: document.archived,
        title: document.title,
        icon: document.icon,
        summary: document.summary,
        content: document.content,
      }),
    )
    .digest('hex');
}
