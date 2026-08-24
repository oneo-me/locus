import { type ComarkNode, type ComarkPlugin, parse } from 'comark';
import type { SerializedMarkdownTree } from '~/contracts/library-api';
import type { ImageSize } from '~/contracts/library-types';
import { createCanvasPlugin } from '~/components/markdown/canvas';
import { createAttachmentViewsPlugin } from './attachment-views';
import { createWikiLinkPlugin } from './wiki-link';

const allowedElementNames = new Set([
  'a',
  'asterweft-wiki-link',
  'attachments',
  'blockquote',
  'br',
  'canvas',
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
]);

function sanitizeNode(node: ComarkNode) {
  if (!Array.isArray(node) || typeof node[0] !== 'string') return;
  if (node[1].id === '') delete node[1].id;
  if (!allowedElementNames.has(node[0])) {
    node[1]['data-unsupported-markdown-component'] = node[0];
    node[0] = 'div';
  }
  for (const child of node.slice(2) as ComarkNode[]) sanitizeNode(child);
}

const safeElementsPlugin: ComarkPlugin = {
  name: 'asterweft-safe-elements',
  post(state) {
    if (!state.tree) return;
    for (const node of state.tree.nodes) sanitizeNode(node);
  },
};

export async function parseDocumentMarkdown({
  assetImageSizes,
  assets,
  content,
  documentPath,
}: {
  assetImageSizes: Record<string, ImageSize>;
  assets: string[];
  content: string;
  documentPath: string;
}): Promise<SerializedMarkdownTree> {
  return parse(content, {
    autoUnwrap: false,
    html: false,
    linkify: false,
    plugins: [
      createWikiLinkPlugin(),
      createCanvasPlugin(),
      createAttachmentViewsPlugin({
        assetImageSizes,
        assets,
        documentPath,
      }),
      safeElementsPlugin,
    ],
  });
}
