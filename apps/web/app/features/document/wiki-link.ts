import { parseWikiLinkArticlePath } from '~/contracts/wiki-link-path';
import {
  urlPathToDocumentPath,
  urlPathToHref,
} from '~/contracts/library-route';
import type { ComarkPlugin, MarkdownItPlugin } from 'comark';

export interface WikiLinkDocument {
  path: string;
  title: string;
  icon?: string | null;
  iconImageModifiedAt?: string | null;
}

export interface ResolvedWikiLink {
  articlePath: string;
  documentPath: string;
  href: string;
  icon: string | null;
  iconImageModifiedAt: string | null;
  label: string;
  titlePath: string;
}

/** 由导航文档构建 path → 文档映射。 */
export function createWikiLinkDocumentMap(
  documents: readonly WikiLinkDocument[],
): Map<string, WikiLinkDocument> {
  return new Map(documents.map((document) => [document.path, document]));
}

/** 用祖先到自身的 title 拼出完整路径。 */
export function wikiLinkTitlePath(
  articlePath: string,
  documentsByPath: ReadonlyMap<string, WikiLinkDocument>,
): string {
  if (!articlePath) {
    return documentsByPath.get('index.md')?.title ?? 'index.md';
  }

  const parts: string[] = [];
  let current = '';
  for (const segment of articlePath.split('/')) {
    current = current ? `${current}/${segment}` : segment;
    const title = documentsByPath.get(`${current}/index.md`)?.title;
    parts.push(title ?? segment);
  }
  return parts.join(' / ');
}

export function resolveWikiLink(
  rawPath: string,
  documentsByPath: ReadonlyMap<string, WikiLinkDocument>,
): ResolvedWikiLink | null {
  const articlePath = parseWikiLinkArticlePath(rawPath);
  if (articlePath === null) return null;

  const documentPath = urlPathToDocumentPath(articlePath);
  const document = documentsByPath.get(documentPath);
  if (!document) return null;

  return {
    articlePath,
    documentPath,
    href: urlPathToHref(articlePath),
    icon: document.icon ?? null,
    iconImageModifiedAt: document.iconImageModifiedAt ?? null,
    label: document.title,
    titlePath: wikiLinkTitlePath(articlePath, documentsByPath),
  };
}

function wikiLinkInlineRule(
  state: {
    pos: number;
    posMax: number;
    src: string;
    push(
      type: string,
      tag: string,
      nesting: number,
    ): { attrs: Array<[string, string]> | null; content: string };
  },
  silent: boolean,
): boolean {
  const start = state.pos;
  if (start + 3 >= state.posMax) return false;
  if (state.src.charCodeAt(start) !== 0x5b /* [ */) return false;
  if (state.src.charCodeAt(start + 1) !== 0x5b /* [ */) return false;

  let pos = start + 2;
  while (pos < state.posMax) {
    const code = state.src.charCodeAt(pos);
    if (code === 0x0a /* \n */) return false;
    if (code === 0x5d /* ] */) {
      if (pos + 1 >= state.posMax) return false;
      if (state.src.charCodeAt(pos + 1) !== 0x5d /* ] */) return false;

      const rawPath = state.src.slice(start + 2, pos);
      if (parseWikiLinkArticlePath(rawPath) === null) return false;
      if (!silent) {
        const token = state.push(
          'mdc_inline_component',
          'asterweft-wiki-link',
          0,
        );
        token.attrs = [['path', rawPath.trim()]];
        token.content = rawPath.trim();
      }
      state.pos = pos + 2;
      return true;
    }
    pos += 1;
  }
  return false;
}

const markdownItWikiLink: MarkdownItPlugin = (markdown) => {
  markdown.inline.ruler.before(
    'comark_inline_span',
    'asterweft_wiki_link',
    wikiLinkInlineRule,
  );
};

export function createWikiLinkPlugin(): ComarkPlugin {
  return {
    name: 'asterweft-wiki-link',
    markdownItPlugins: [markdownItWikiLink],
  };
}
