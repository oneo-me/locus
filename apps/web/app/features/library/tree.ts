import type { LibraryNavigationDocument } from '~/contracts/library-api';

export interface DocumentTreeNode {
  name: string;
  path: string;
  document: LibraryNavigationDocument | null;
  children: DocumentTreeNode[];
}

interface MutableDocumentTreeNode {
  name: string;
  path: string;
  document: LibraryNavigationDocument | null;
  children: Map<string, MutableDocumentTreeNode>;
}

export function includeSearchResultAncestors(
  results: LibraryNavigationDocument[],
  documents: LibraryNavigationDocument[],
): LibraryNavigationDocument[] {
  const documentsByDirectory = new Map(
    documents.map((document) => [document.directory, document]),
  );
  const includedPaths = new Set(results.map((document) => document.path));

  for (const result of results) {
    let directory = result.directory;
    while (directory) {
      const ancestor = documentsByDirectory.get(directory);
      if (ancestor) includedPaths.add(ancestor.path);
      directory = directory.split('/').slice(0, -1).join('/');
    }
  }

  const includedDocuments = documents.filter((document) =>
    includedPaths.has(document.path),
  );
  const knownPaths = new Set(
    includedDocuments.map((document) => document.path),
  );

  return [
    ...includedDocuments,
    ...results.filter((document) => !knownPaths.has(document.path)),
  ];
}

export function buildDocumentTree(
  documents: LibraryNavigationDocument[],
  directories: string[] = [],
  order: Readonly<Record<string, number>> = {},
): DocumentTreeNode[] {
  const root: MutableDocumentTreeNode = {
    name: '',
    path: '',
    document: null,
    children: new Map(),
  };

  for (const directory of directories) ensureDirectory(root, directory);
  for (const document of documents) {
    ensureDirectory(root, document.directory).document = document;
  }

  const children = finalizeChildren(root, order);
  if (!root.document) return children;

  return [
    {
      name: root.document.title,
      path: root.document.path,
      document: root.document,
      children: [],
    },
    ...children,
  ].sort((left, right) => compareDocumentTreeNodes(left, right, order));
}

export function getOpenNodePaths(
  selectionPath: string,
  directories: string[],
): string[] {
  const nodePath = directories.includes(selectionPath)
    ? selectionPath
    : selectionPath.split('/').slice(0, -1).join('/');
  const segments = nodePath.split('/').filter(Boolean);
  return segments.map((_, index) => segments.slice(0, index + 1).join('/'));
}

function ensureDirectory(
  root: MutableDocumentTreeNode,
  directory: string,
): MutableDocumentTreeNode {
  let current = root;
  let currentPath = '';

  for (const name of directory.split('/').filter(Boolean)) {
    currentPath = currentPath ? `${currentPath}/${name}` : name;
    let child = current.children.get(name);
    if (!child) {
      child = {
        name,
        path: currentPath,
        document: null,
        children: new Map(),
      };
      current.children.set(name, child);
    }
    current = child;
  }

  return current;
}

function finalizeChildren(
  node: MutableDocumentTreeNode,
  order: Readonly<Record<string, number>>,
): DocumentTreeNode[] {
  return [...node.children.values()]
    .sort((left, right) => compareDocumentTreeNodes(left, right, order))
    .map((child) => finalizeNode(child, order));
}

function finalizeNode(
  node: MutableDocumentTreeNode,
  order: Readonly<Record<string, number>>,
): DocumentTreeNode {
  return {
    name: node.name,
    path: node.path,
    document: node.document,
    children: finalizeChildren(node, order),
  };
}

interface SortableDocumentTreeNode {
  path: string;
  document: LibraryNavigationDocument | null;
  children: { size: number } | { length: number };
}

function compareDocumentTreeNodes(
  left: SortableDocumentTreeNode,
  right: SortableDocumentTreeNode,
  order: Readonly<Record<string, number>>,
): number {
  const leftOrder = left.document ? order[left.document.path] : undefined;
  const rightOrder = right.document ? order[right.document.path] : undefined;
  const leftRank =
    typeof leftOrder === 'number' && Number.isFinite(leftOrder)
      ? leftOrder
      : Number.POSITIVE_INFINITY;
  const rightRank =
    typeof rightOrder === 'number' && Number.isFinite(rightOrder)
      ? rightOrder
      : Number.POSITIVE_INFINITY;
  if (leftRank !== rightRank) return leftRank - rightRank;

  const leftIsRoot = left.document?.directory === '';
  const rightIsRoot = right.document?.directory === '';
  if (leftIsRoot !== rightIsRoot) return leftIsRoot ? -1 : 1;

  const leftHasChildren = nodeHasChildren(left);
  const rightHasChildren = nodeHasChildren(right);
  if (leftHasChildren !== rightHasChildren) {
    return leftHasChildren ? -1 : 1;
  }

  return left.path.localeCompare(right.path);
}

function nodeHasChildren(node: SortableDocumentTreeNode): boolean {
  return 'size' in node.children
    ? node.children.size > 0
    : node.children.length > 0;
}
