import type { DocumentGraph } from '~/contracts/document-graph';
import { describe, expect, test } from 'vitest';
import { filterDocumentGraph } from './filter';

const sampleGraph: DocumentGraph = {
  nodes: [
    {
      id: 'doc:a',
      kind: 'document',
      path: 'a/index.md',
      title: 'A',
      directory: 'a',
      inboundCount: 0,
      outboundCount: 1,
      degree: 1,
    },
    {
      id: 'doc:b',
      kind: 'document',
      path: 'a/b/index.md',
      title: 'B',
      directory: 'a/b',
      inboundCount: 1,
      outboundCount: 0,
      degree: 1,
    },
    {
      id: 'doc:c',
      kind: 'document',
      path: 'c/index.md',
      title: 'C',
      directory: 'c',
      inboundCount: 1,
      outboundCount: 0,
      degree: 1,
    },
    {
      id: 'attachment:a:note.pdf',
      kind: 'attachment',
      path: 'a/note.pdf',
      title: 'note.pdf',
      directory: 'a',
      inboundCount: 1,
      outboundCount: 0,
      degree: 1,
    },
  ],
  edges: [
    {
      id: 'hierarchy:doc:a->doc:b',
      kind: 'hierarchy',
      source: 'doc:a',
      target: 'doc:b',
      count: 1,
    },
    {
      id: 'reference:doc:a->doc:c',
      kind: 'reference',
      source: 'doc:a',
      target: 'doc:c',
      count: 1,
    },
    {
      id: 'attachment:doc:a->attachment:a:note.pdf',
      kind: 'attachment',
      source: 'doc:a',
      target: 'attachment:a:note.pdf',
      count: 1,
    },
  ],
};

describe('filterDocumentGraph', () => {
  test('keeps the graph unchanged when attachments are visible', () => {
    expect(
      filterDocumentGraph(sampleGraph, {
        showAttachments: true,
      }),
    ).toBe(sampleGraph);
  });

  test('removes attachment nodes and edges while keeping references', () => {
    expect(
      filterDocumentGraph(sampleGraph, {
        showAttachments: false,
      }),
    ).toEqual({
      nodes: [sampleGraph.nodes[0], sampleGraph.nodes[1], sampleGraph.nodes[2]],
      edges: [sampleGraph.edges[0], sampleGraph.edges[1]],
    });
  });
});
