import { describe, expect, test } from 'vitest';
import type { DocumentTreeNode } from '../library/tree';
import { buildTreeReorderItems, reorderTreeNodes } from './tree-reorder';

function node(path: string, documentPath?: string): DocumentTreeNode {
  return {
    name: path,
    path,
    document: documentPath
      ? {
          attachmentCount: 0,
          path: documentPath,
          directory: path,
          title: path,
          icon: null,
          iconImageModifiedAt: null,
        }
      : null,
    children: [],
  };
}

describe('tree reorder helpers', () => {
  test('reorders nodes within the same list', () => {
    const nodes = [node('a', 'a/index.md'), node('b', 'b/index.md'), node('c')];
    expect(reorderTreeNodes(nodes, 0, 2).map((item) => item.path)).toEqual([
      'b',
      'c',
      'a',
    ]);
  });

  test('builds order payloads only for document nodes', () => {
    const nodes = [
      node('a', 'a/index.md'),
      node('empty'),
      node('b', 'b/index.md'),
    ];
    expect(buildTreeReorderItems(nodes)).toEqual([
      { path: 'a/index.md', order: 0 },
      { path: 'b/index.md', order: 2 },
    ]);
  });
});
