import { describe, expect, test } from 'vitest';
import { type GraphEventSnapshot, parseGraphEventSnapshot } from './events';

describe('graph event snapshots', () => {
  test('parses a graph snapshot', () => {
    const snapshot: GraphEventSnapshot = {
      status: {
        rootPath: '/docs',
        cachePath: '/docs/cache.sqlite',
        documentCount: 1,
        watching: true,
        refreshing: false,
        indexedAt: '2026-07-24T00:00:00.000Z',
      },
      graph: {
        nodes: [],
        edges: [],
      },
      navigation: {
        directories: [],
        documents: [],
        search: null,
      },
      repositoryChanges: [],
      repositoryError: null,
      repositoryRevision: 'revision-1',
    };

    expect(parseGraphEventSnapshot(JSON.stringify(snapshot))).toEqual(snapshot);
  });

  test('parses a snapshot without article content', () => {
    const snapshot: GraphEventSnapshot = {
      status: {
        rootPath: '/docs',
        cachePath: '/docs/cache.sqlite',
        documentCount: 1,
        watching: true,
        refreshing: false,
        indexedAt: '2026-07-24T00:00:00.000Z',
      },
      graph: {
        nodes: [],
        edges: [],
      },
      navigation: {
        directories: [],
        documents: [],
        search: null,
      },
      repositoryChanges: [
        {
          file: 'notes/index.md',
          status: 'modified',
          staged: false,
          unstaged: true,
        },
      ],
      repositoryError: null,
      repositoryRevision: 'revision-2',
    };

    expect(parseGraphEventSnapshot(JSON.stringify(snapshot))).toEqual(snapshot);
  });

  test('ignores malformed messages', () => {
    expect(parseGraphEventSnapshot('not json')).toBeNull();
    expect(parseGraphEventSnapshot('{"graph":{}}')).toBeNull();
    expect(
      parseGraphEventSnapshot(
        JSON.stringify({
          status: {
            rootPath: '/docs',
            cachePath: '/docs/cache.sqlite',
            documentCount: 1,
            watching: true,
            refreshing: false,
            indexedAt: null,
          },
          graph: { nodes: [], edges: [] },
          navigation: { directories: [], documents: [], search: null },
          repositoryChanges: [],
          repositoryError: null,
          repositoryRevision: 1,
        }),
      ),
    ).toBeNull();
  });
});
