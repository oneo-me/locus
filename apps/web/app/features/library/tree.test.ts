import type { LibraryNavigationDocument } from '~/contracts/library-api';
import { describe, expect, test } from 'vitest';
import {
  buildDocumentTree,
  getOpenNodePaths,
  includeSearchResultAncestors,
} from './tree';

const summaryDefaults = {
  attachmentCount: 0,
  icon: null,
  iconImageModifiedAt: null,
};

function document(path: string, title: string): LibraryNavigationDocument {
  return {
    ...summaryDefaults,
    path,
    directory: path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : '',
    title,
  };
}

describe('buildDocumentTree', () => {
  test('keeps the root index beside top-level document nodes', () => {
    const tree = buildDocumentTree([
      document('guide/setup/index.md', '安装'),
      document('index.md', '首页'),
      document('reference/index.md', '参考'),
      document('guide/index.md', '指南'),
    ]);

    expect(tree).toHaveLength(3);
    expect(tree[0]?.document?.path).toBe('index.md');
    expect(tree[0]?.children).toEqual([]);
    expect(tree.slice(1).map(({ document }) => document?.path)).toEqual([
      'guide/index.md',
      'reference/index.md',
    ]);
    expect(tree[1]?.children[0]?.document?.path).toBe('guide/setup/index.md');
  });

  test('preserves directories whose intermediate directory has no index', () => {
    const tree = buildDocumentTree(
      [
        document('guide/index.md', '指南'),
        document('misc/deep/index.md', '深层文档'),
      ],
      ['empty', 'guide', 'misc', 'misc/deep'],
    );

    expect(tree.map(({ path }) => path)).toEqual(['misc', 'empty', 'guide']);
    expect(tree[0]).toMatchObject({
      name: 'misc',
      path: 'misc',
      document: null,
    });
    expect(tree[0]?.children[0]?.document?.path).toBe('misc/deep/index.md');
    expect(tree[1]).toMatchObject({
      name: 'empty',
      path: 'empty',
      document: null,
      children: [],
    });
    expect(tree[2]).toMatchObject({
      name: 'guide',
      path: 'guide',
      document: {
        path: 'guide/index.md',
        title: '指南',
      },
    });
  });

  test('sorts nodes with children before leaf nodes', () => {
    const tree = buildDocumentTree(
      [
        document('alpha/index.md', 'Alpha'),
        document('beta/leaf/index.md', 'Beta 叶子'),
        document('beta/index.md', 'Beta'),
        document('gamma/index.md', 'Gamma'),
      ],
      ['alpha', 'beta', 'beta/leaf'],
    );

    expect(tree.map(({ path }) => path)).toEqual(['beta', 'alpha', 'gamma']);
    expect(tree[0]?.children[0]?.document?.path).toBe('beta/leaf/index.md');
  });

  test('sorts siblings using the independent tree state', () => {
    const tree = buildDocumentTree(
      [
        document('alpha/index.md', 'Alpha'),
        document('beta/index.md', 'Beta'),
        document('index.md', '首页'),
        document('zeta/index.md', 'Zeta'),
      ],
      [],
      {
        'beta/index.md': 1,
        'zeta/index.md': 0,
      },
    );

    expect(tree.map((node) => node.document?.path)).toEqual([
      'zeta/index.md',
      'beta/index.md',
      'index.md',
      'alpha/index.md',
    ]);
  });
});

describe('includeSearchResultAncestors', () => {
  test('keeps ancestor article titles and icons in a filtered search tree', () => {
    const projects = {
      ...document('projects/index.md', '项目'),
      icon: '🔥',
    };
    const asterweft = document('projects/asterweft/index.md', 'Asterweft');
    const documents = [
      document('health/index.md', '健康'),
      projects,
      asterweft,
    ];

    const visibleDocuments = includeSearchResultAncestors(
      [asterweft],
      documents,
    );
    const tree = buildDocumentTree(visibleDocuments);

    expect(visibleDocuments.map(({ path }) => path)).toEqual([
      'projects/index.md',
      'projects/asterweft/index.md',
    ]);
    expect(tree).toHaveLength(1);
    expect(tree[0]).toMatchObject({
      document: {
        icon: '🔥',
        title: '项目',
      },
    });
    expect(tree[0]?.children[0]?.document?.title).toBe('Asterweft');
  });
});

describe('getOpenNodePaths', () => {
  test('includes the selected node so nodes with children open automatically', () => {
    const directories = ['guide', 'guide/setup'];
    expect(getOpenNodePaths('guide/setup/index.md', directories)).toEqual([
      'guide',
      'guide/setup',
    ]);
    expect(getOpenNodePaths('guide', directories)).toEqual(['guide']);
    expect(getOpenNodePaths('index.md', directories)).toEqual([]);
  });
});
