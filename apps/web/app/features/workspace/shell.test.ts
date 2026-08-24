import type { LibraryWorkspaceView } from '~/contracts/library-api';
import { describe, expect, test } from 'vitest';
import type { GraphEventSnapshot } from '../graph/events';
import {
  createWorkspaceEventPath,
  mergeWorkspaceLibraryData,
  type WorkspaceSnapshotState,
} from './shell';

function libraryData(title: string): LibraryWorkspaceView {
  const document = {
    attachmentCount: 0,
    directory: 'notes',
    icon: null,
    iconImageModifiedAt: null,
    path: 'notes/index.md',
    title,
  };
  return {
    directories: ['notes'],
    documents: [document],
    repositoryChanges: [],
    repositoryError: null,
    repositoryRevision: 'revision-1',
    search: null,
    status: {
      cachePath: '/cache',
      documentCount: 1,
      indexedAt: '2026-07-28T00:00:00.000Z',
      refreshing: false,
      rootPath: '/notes',
      watching: true,
    },
    libraryState: { graphParticipation: {}, order: {} },
  };
}

function snapshotState(
  title: string,
  query: string | null = null,
): WorkspaceSnapshotState {
  const document = {
    attachmentCount: 0,
    directory: 'notes',
    icon: null,
    iconImageModifiedAt: null,
    path: 'notes/index.md',
    title,
  };
  return {
    query,
    snapshot: {
      navigation: {
        directories: ['notes'],
        documents: [document],
        search: null,
      },
      repositoryChanges: [],
      repositoryError: null,
      repositoryRevision: 'revision-2',
      status: {
        cachePath: '/cache',
        documentCount: 1,
        indexedAt: '2026-07-28T00:00:01.000Z',
        refreshing: false,
        rootPath: '/notes',
        watching: true,
      },
    } as GraphEventSnapshot,
  };
}

describe('workspace live library data', () => {
  test('keeps the event stream URL independent from article paths', () => {
    expect(createWorkspaceEventPath('documents', null)).toBe(
      '/api/graph-events',
    );
    expect(createWorkspaceEventPath('documents', '新')).toBe(
      '/api/graph-events?q=%E6%96%B0',
    );
  });

  test('applies live navigation and repository data', () => {
    const data = libraryData('旧名称');
    const state = snapshotState('新名称');
    state.snapshot.repositoryChanges = [
      {
        file: 'notes/index.md',
        staged: false,
        status: 'modified',
        unstaged: true,
      },
    ];

    const merged = mergeWorkspaceLibraryData(data, state, null);

    expect(merged.documents[0]?.title).toBe('新名称');
    expect(merged.repositoryRevision).toBe('revision-2');
    expect(merged.repositoryChanges).toEqual(state.snapshot.repositoryChanges);
  });

  test('applies search results only for the current query', () => {
    const data = libraryData('旧名称');
    const state = snapshotState('新名称', '新');
    state.snapshot.navigation.search = {
      query: '新',
      total: 1,
      results: state.snapshot.navigation.documents,
    };

    expect(mergeWorkspaceLibraryData(data, state, '新').search?.total).toBe(1);
    expect(mergeWorkspaceLibraryData(data, state, '其他').search).toBeNull();
  });

  test('keeps the loader object when the first live snapshot is identical', () => {
    const data = libraryData('当前名称');
    const state = snapshotState('当前名称');
    state.snapshot.status = data.status;
    state.snapshot.navigation.directories = data.directories;
    state.snapshot.navigation.documents = data.documents;
    state.snapshot.navigation.search = data.search;
    state.snapshot.repositoryChanges = data.repositoryChanges;
    state.snapshot.repositoryError = data.repositoryError;
    state.snapshot.repositoryRevision = data.repositoryRevision;

    expect(mergeWorkspaceLibraryData(data, state, null)).toBe(data);
  });
});
