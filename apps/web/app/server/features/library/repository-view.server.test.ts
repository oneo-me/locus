import { describe, expect, test, vi } from 'vitest';
import type { DocumentLibrary } from './core/document-library';
import { readLibraryRepositoryView } from './repository-view.server';

describe('library repository view', () => {
  test('returns repository changes without an error', () => {
    const repositoryChanges = {
      revision: 'revision',
      changes: [
        {
          file: 'notes/index.md',
          status: 'modified' as const,
          staged: false,
          unstaged: true,
        },
      ],
    };
    const library = {
      repositoryChanges: () => repositoryChanges,
    } as unknown as DocumentLibrary;

    expect(readLibraryRepositoryView(library)).toEqual({
      ...repositoryChanges,
      error: null,
    });
  });

  test('skips Git entirely when disabled and presents a clean view', () => {
    const originalGitDisabled = process.env.ASTERWEFT_GIT_DISABLED;
    process.env.ASTERWEFT_GIT_DISABLED = '1';
    const library = {
      repositoryChanges: () => {
        throw new Error('文档库目录必须是 Git 仓库根目录。');
      },
    } as unknown as DocumentLibrary;
    const consoleError = vi.fn(() => {});
    const originalConsoleError = console.error;
    console.error = consoleError;

    try {
      expect(readLibraryRepositoryView(library)).toEqual({
        revision: null,
        changes: [],
        error: null,
      });
      expect(consoleError).not.toHaveBeenCalled();
    } finally {
      if (originalGitDisabled === undefined) {
        delete process.env.ASTERWEFT_GIT_DISABLED;
      } else {
        process.env.ASTERWEFT_GIT_DISABLED = originalGitDisabled;
      }
      console.error = originalConsoleError;
    }
  });

  test('reports Git failures instead of presenting an empty repository', () => {
    const error = new Error('文档库目录不是 Git 仓库。');
    const library = {
      repositoryChanges: () => {
        throw error;
      },
    } as unknown as DocumentLibrary;
    const consoleError = vi.fn(() => {});
    const originalConsoleError = console.error;
    console.error = consoleError;

    try {
      expect(readLibraryRepositoryView(library)).toEqual({
        revision: null,
        changes: [],
        error: error.message,
      });
      readLibraryRepositoryView(library);
      expect(consoleError).toHaveBeenCalledWith(
        '无法读取文档库 Git 状态。',
        error,
      );
      expect(consoleError).toHaveBeenCalledTimes(1);
    } finally {
      console.error = originalConsoleError;
    }
  });
});
