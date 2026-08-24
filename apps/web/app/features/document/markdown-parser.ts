import { type ComarkPlugin, type Node, parseMarkdown } from 'comark';
import alert from 'comark/plugins/alert';
import attributes from 'comark/plugins/attributes';
import components from 'comark/plugins/components';
import frontmatter from 'comark/plugins/frontmatter';
import taskList from 'comark/plugins/task-list';
import type {
  LibrarySelectionSourceView,
  LibrarySelectionView,
  SerializedMarkdownTree,
} from '@locus/document-library/contracts/library-api';
import type { ImageSize } from '@locus/document-library/contracts/library-types';
import { createCanvasPlugin } from '../../components/markdown/canvas';
import { createAttachmentViewsPlugin } from './attachment-views';
import { createWikiLinkPlugin } from './wiki-link';

const allowedElementNames = new Set([
  'a',
  'locus-wiki-link',
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

function sanitizeNode(node: Node) {
  if (!Array.isArray(node) || typeof node[0] !== 'string') return;
  if (node[1].id === '') delete node[1].id;
  if (!allowedElementNames.has(node[0])) {
    node[1]['data-unsupported-markdown-component'] = node[0];
    node[0] = 'div';
  }
  for (const child of node.slice(2) as Node[]) sanitizeNode(child);
}

const safeElementsPlugin: ComarkPlugin = {
  name: 'locus-safe-elements',
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

export async function prepareLibrarySelectionView(
  source: LibrarySelectionSourceView,
): Promise<LibrarySelectionView> {
  const { assetImageSizes, selected, ...selection } = source;
  if (!selected) return { ...selection, selected: null };
  return {
    ...selection,
    selected: {
      ...selected,
      content: await parseDocumentMarkdown({
        assetImageSizes,
        assets: selected.assets,
        content: selected.content,
        documentPath: selected.path,
      }),
    },
  };
}
