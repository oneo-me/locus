import { describe, expect, test } from 'vitest';
import { buildDocumentGraph } from './document-graph.js';
import type { DocumentContent } from './types.js';

function document(path: string, title: string, content = ''): DocumentContent {
  const directory = path.includes('/')
    ? path.slice(0, path.lastIndexOf('/'))
    : '';
  return {
    path,
    directory,
    title,
    icon: null,
    iconImageModifiedAt: null,
    metadataKeys: ['title'],
    summary: null,
    assets: [],
    modifiedAt: '2026-07-23T00:00:00.000Z',
    size: content.length,
    wordCount: 0,
    issue: null,
    content,
  };
}

describe('document graph', () => {
  test('adds direct parent-child hierarchy edges and degrees', () => {
    const graph = buildDocumentGraph([
      document('index.md', '首页'),
      document('guide/index.md', '指南'),
      document('guide/setup/index.md', '安装'),
      document('orphan/deep/index.md', '缺少父文章'),
    ]);

    expect(graph.edges.filter((edge) => edge.kind === 'hierarchy')).toEqual([
      {
        id: 'hierarchy-edge-0',
        kind: 'hierarchy',
        source: 'guide/index.md',
        target: 'guide/setup/index.md',
        count: 1,
      },
      {
        id: 'hierarchy-edge-1',
        kind: 'hierarchy',
        source: 'index.md',
        target: 'guide/index.md',
        count: 1,
      },
    ]);
    expect(
      graph.nodes
        .filter((node) => node.kind === 'document')
        .map((node) => ({
          path: node.path,
          inboundCount: node.inboundCount,
          outboundCount: node.outboundCount,
          degree: node.degree,
        })),
    ).toEqual([
      {
        path: 'guide/index.md',
        inboundCount: 1,
        outboundCount: 1,
        degree: 2,
      },
      {
        path: 'guide/setup/index.md',
        inboundCount: 1,
        outboundCount: 0,
        degree: 1,
      },
      {
        path: 'index.md',
        inboundCount: 0,
        outboundCount: 1,
        degree: 1,
      },
      {
        path: 'orphan/deep/index.md',
        inboundCount: 0,
        outboundCount: 0,
        degree: 0,
      },
    ]);
  });

  test('builds reference edges from wiki links and ignores markdown links', () => {
    const graph = buildDocumentGraph([
      document(
        'guide/index.md',
        '指南',
        [
          '[配置](./setup)',
          '[[architecture]]',
          '[[architecture]]',
          '[[missing]]',
          '[外部](https://example.com)',
        ].join('\n'),
      ),
      document('guide/setup/index.md', '配置'),
      document('architecture/index.md', '架构设计'),
    ]);

    expect(graph.edges.filter((edge) => edge.kind === 'hierarchy')).toEqual([
      {
        id: 'hierarchy-edge-0',
        kind: 'hierarchy',
        source: 'guide/index.md',
        target: 'guide/setup/index.md',
        count: 1,
      },
    ]);
    expect(graph.edges.filter((edge) => edge.kind === 'reference')).toEqual([
      {
        id: 'reference-edge-0',
        kind: 'reference',
        source: 'guide/index.md',
        target: 'architecture/index.md',
        count: 2,
      },
    ]);
    expect(
      graph.nodes.find((node) => node.path === 'guide/index.md'),
    ).toMatchObject({
      inboundCount: 0,
      outboundCount: 2,
      degree: 2,
    });
    expect(
      graph.nodes.find((node) => node.path === 'architecture/index.md'),
    ).toMatchObject({
      inboundCount: 1,
      outboundCount: 0,
      degree: 1,
    });
  });

  test('skips wiki links to excluded or self documents', () => {
    const graph = buildDocumentGraph(
      [
        document('guide/index.md', '指南', '[[guide]] [[architecture]]'),
        document('architecture/index.md', '架构设计'),
      ],
      new Set(['architecture/index.md']),
    );

    expect(graph.edges.filter((edge) => edge.kind === 'reference')).toEqual([]);
  });

  test('adds discovered attachments as owner-scoped graph nodes', () => {
    const owner = document('guide/index.md', '指南');
    owner.assets = ['diagram.png', 'brief.pdf', 'diagram.png'];
    const graph = buildDocumentGraph([owner]);

    expect(graph.nodes).toEqual([
      expect.objectContaining({ id: 'guide/index.md', degree: 2 }),
      expect.objectContaining({
        id: 'attachment:guide%2Findex.md:brief.pdf',
        kind: 'attachment',
        path: 'guide/brief.pdf',
        title: 'brief.pdf',
        degree: 1,
      }),
      expect.objectContaining({
        id: 'attachment:guide%2Findex.md:diagram.png',
        kind: 'attachment',
        path: 'guide/diagram.png',
        title: 'diagram.png',
        degree: 1,
      }),
    ]);
    expect(graph.edges).toEqual([
      {
        id: 'attachment-edge-0',
        kind: 'attachment',
        source: 'guide/index.md',
        target: 'attachment:guide%2Findex.md:brief.pdf',
        count: 1,
      },
      {
        id: 'attachment-edge-1',
        kind: 'attachment',
        source: 'guide/index.md',
        target: 'attachment:guide%2Findex.md:diagram.png',
        count: 1,
      },
    ]);
  });
});
