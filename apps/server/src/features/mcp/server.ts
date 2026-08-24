import { join } from 'node:path';
import { McpServer } from '@modelcontextprotocol/server';
import { articleMetadata } from '@locus/document-library/server/article-content';
import {
  articlePathToDocumentPath,
  normalizeArticlePath,
} from '@locus/document-library/server/article-paths';
import type { ArticleListItem } from '@locus/document-library/server/article-types';
import type { DocumentLibrary } from '@locus/document-library/server/document-library';
import { documentIndexName } from '@locus/document-library/server/paths';
import * as z from 'zod/v4';
import { handleMcpToolCall } from './tool-errors';

const parentArticlePathSchema = z
  .string()
  .trim()
  .default('')
  .refine(
    (path) =>
      path === '' || path.split('/').some((segment) => segment.length > 0),
    'Use an empty string only to list top-level articles.',
  )
  .describe(
    'Parent article identifier. An empty string represents the top level.',
  );

const resolveArticlePathSchema = z
  .string()
  .trim()
  .min(1)
  .describe(
    'Library-relative article path. A trailing /index.md segment is accepted and normalized away.',
  );

const mcpInstructions = [
  'Locus exposes list_articles, search_articles, and resolve_article for discovering a local Markdown library.',
  'If the user provides a library-relative article path, call resolve_article directly; do not list or search first.',
  'Use search_articles for titles or keywords; use list_articles to browse children.',
  'Each result includes an article path for hierarchy navigation plus absolute file and directory paths.',
  'Edit content with the client file tools: each article is a directory containing index.md;',
  'YAML frontmatter holds title, icon, and summary; sibling regular files are attachments.',
  'Do not recreate CRUD or attachment tools through MCP.',
].join(' ');

function toolResult(value: unknown) {
  return {
    content: [{ type: 'text' as const, text: JSON.stringify(value, null, 2) }],
    structuredContent: value,
  };
}

function normalizeResolveArticlePath(path: string): string {
  const normalized = path.replaceAll('\\', '/').replace(/^\/+|\/+$/g, '');
  const indexSuffix = `/${documentIndexName}`;
  if (normalized === documentIndexName) return '';
  if (normalized.endsWith(indexSuffix)) {
    return normalizeArticlePath(normalized.slice(0, -indexSuffix.length));
  }
  return normalizeArticlePath(normalized);
}

function articleLocalPaths(rootPath: string, articlePath: string) {
  const documentPath = articlePathToDocumentPath(articlePath);
  const file = join(rootPath, ...documentPath.split('/'));
  const directory = join(
    rootPath,
    ...articlePath.split('/').filter((segment) => segment.length > 0),
  );
  return { file, directory };
}

function withLocalPaths(
  rootPath: string,
  article: ArticleListItem | ReturnType<typeof articleMetadata>,
) {
  return {
    ...article,
    ...articleLocalPaths(rootPath, article.path),
  };
}

export function createLocusMcpServer(library: DocumentLibrary): McpServer {
  const rootPath = library.getRootPath();
  const server = new McpServer(
    {
      name: 'locus',
      version: '0.1.0',
    },
    {
      supportedProtocolVersions: ['2026-07-28', '2025-11-25'],
      instructions: mcpInstructions,
    },
  );

  server.registerTool(
    'list_articles',
    {
      title: 'List child articles',
      description:
        'Lists direct child articles for a parent identifier with cursor pagination. Each result includes article metadata, Markdown body lineCount, attachmentCount, childCount, and absolute local file and directory paths for native editing.',
      inputSchema: z
        .object({
          parent: parentArticlePathSchema,
          cursor: z.number().int().min(0).default(0),
          limit: z.number().int().min(1).max(200).default(50),
        })
        .strict(),
      annotations: {
        readOnlyHint: true,
        destructiveHint: false,
        idempotentHint: true,
        openWorldHint: false,
      },
    },
    async ({ parent, cursor, limit }) =>
      handleMcpToolCall(() => {
        const articles = library.listChildArticles(parent);
        const page = articles.slice(cursor, cursor + limit);
        return toolResult({
          parent,
          articles: page.map((article) => withLocalPaths(rootPath, article)),
          nextCursor:
            cursor + page.length < articles.length
              ? cursor + page.length
              : null,
        });
      }),
  );

  server.registerTool(
    'search_articles',
    {
      title: 'Search articles',
      description:
        'Searches Markdown article titles, public metadata, and body content. Each result includes attachmentCount plus absolute local file and directory paths for native editing.',
      inputSchema: z
        .object({
          query: z.string().trim().min(1),
          limit: z.number().int().min(1).max(100).default(20),
        })
        .strict(),
      annotations: {
        readOnlyHint: true,
        destructiveHint: false,
        idempotentHint: true,
        openWorldHint: false,
      },
    },
    async ({ query, limit }) =>
      handleMcpToolCall(() => {
        const result = library.searchDocuments(query, limit);
        return toolResult({
          query: result.query,
          total: result.total,
          results: result.results.map((match) => {
            const document = library.getDocument(match.path);
            return {
              ...withLocalPaths(rootPath, articleMetadata(document)),
              excerpt: match.excerpt,
            };
          }),
        });
      }),
  );

  server.registerTool(
    'resolve_article',
    {
      title: 'Resolve an article path',
      description:
        'Resolves a known library-relative article path to public metadata, attachmentCount, and absolute local file and directory paths. Accepts an optional trailing /index.md. Use this instead of search when the path is already known.',
      inputSchema: z
        .object({
          path: resolveArticlePathSchema,
        })
        .strict(),
      annotations: {
        readOnlyHint: true,
        destructiveHint: false,
        idempotentHint: true,
        openWorldHint: false,
      },
    },
    async ({ path: rawPath }) =>
      handleMcpToolCall(() => {
        const path = normalizeResolveArticlePath(rawPath);
        if (!path.split('/').some((segment) => segment.length > 0)) {
          throw new Error('根文章不存在。');
        }
        const document = library.getDocument(articlePathToDocumentPath(path));
        const article = {
          ...articleMetadata(document),
          childCount: library.listChildArticles(path).length,
        };
        return toolResult(withLocalPaths(rootPath, article));
      }),
  );

  return server;
}
