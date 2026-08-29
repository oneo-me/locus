import {
  type ComarkPlugin,
  type MarkdownDocument,
  type Node,
  parseMarkdown,
} from 'comark';
import alert from 'comark/plugins/alert';
import attributes from 'comark/plugins/attributes';
import components from 'comark/plugins/components';
import frontmatter from 'comark/plugins/frontmatter';
import taskList from 'comark/plugins/task-list';
import type { ImageSize } from '@locus/document-library/contracts/library-types';
import type { MarkdownUiExtension } from '@locus/markdown-ui';
import { createAttachmentViewsPlugin } from './attachment-views';
import { createWikiLinkPlugin } from './wiki-link';

const baseAllowedElementNames = [
  'a',
  'locus-wiki-link',
  'attachments',
  'blockquote',
  'br',
  'code',
  'del',
  'em',
  'gallery',
  'h1',
  'h2',
  'h3',
  'h4',
  'h5',
  'h6',
  'hr',
  'img',
  'input',
  'li',
  'ol',
  'p',
  'pre',
  'span',
  'strong',
  'sub',
  'sup',
  'table',
  'tbody',
  'td',
  'th',
  'thead',
  'tr',
  'ul',
];

function sanitizeNode(node: Node, allowedElementNames: ReadonlySet<string>) {
  if (!Array.isArray(node) || typeof node[0] !== 'string') return;
  if (node[1].id === '') delete node[1].id;
  if (!allowedElementNames.has(node[0])) {
    node[1]['data-unsupported-markdown-component'] = node[0];
    node[0] = 'div';
  }
  for (const child of node.slice(2) as Node[]) {
    sanitizeNode(child, allowedElementNames);
  }
}

function createSafeElementsPlugin(
  extensions: readonly MarkdownUiExtension[],
): ComarkPlugin {
  const allowedElementNames = new Set([
    ...baseAllowedElementNames,
    ...extensions.flatMap((extension) => extension.elementNames ?? []),
  ]);
  return {
    name: 'locus-safe-elements',
    post(state) {
      if (!state.tree) return;
      for (const node of state.tree.nodes) {
        sanitizeNode(node, allowedElementNames);
      }
    },
  };
}

export async function parseDocumentMarkdown({
  assetImageSizes,
  assets,
  content,
  documentPath,
  extensions = [],
}: {
  assetImageSizes: Record<string, ImageSize>;
  assets: string[];
  content: string;
  documentPath: string;
  extensions?: readonly MarkdownUiExtension[];
}): Promise<MarkdownDocument> {
  return parseMarkdown(content, {
    autoUnwrap: false,
    linkify: false,
    registerDefaultPlugins: false,
    plugins: [
      frontmatter(),
      alert(),
      taskList(),
      components(),
      attributes(),
      createWikiLinkPlugin(),
      ...extensions.flatMap((extension) => extension.plugins ?? []),
      createAttachmentViewsPlugin({
        assetImageSizes,
        assets,
        documentPath,
      }),
      createSafeElementsPlugin(extensions),
    ],
  });
}
