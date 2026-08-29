import { arrayMove } from '@dnd-kit/sortable';
import type { DocumentTreeNode } from '../library/tree';

export interface TreeReorderItem {
  path: string;
  order: number;
}

export function buildTreeReorderItems(
  nodes: DocumentTreeNode[],
): TreeReorderItem[] {
  return nodes.flatMap((node, index) =>
    node.document ? [{ path: node.document.path, order: index }] : [],
  );
}

export function reorderTreeNodes(
  nodes: DocumentTreeNode[],
  fromIndex: number,
  toIndex: number,
): DocumentTreeNode[] {
  if (
    fromIndex === toIndex ||
    fromIndex < 0 ||
    toIndex < 0 ||
    fromIndex >= nodes.length ||
    toIndex >= nodes.length
  ) {
    return nodes;
  }
  return arrayMove(nodes, fromIndex, toIndex);
}
