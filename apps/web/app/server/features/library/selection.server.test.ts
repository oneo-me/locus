import { describe, expect, test, vi } from 'vitest';
import type { DocumentLibrary } from './core/document-library';
import { resolveLibrarySelection } from './selection.server';

function libraryStub(): DocumentLibrary {
  return {
    getDocument: vi.fn(() => {
      throw new Error('文档不存在。');
    }),
    listAllDirectories: vi.fn(() => []),
  } as unknown as DocumentLibrary;
}

describe('resolveLibrarySelection', () => {
  test('treats the library root as an unselected landing state', () => {
    const library = libraryStub();

    expect(resolveLibrarySelection(library, 'index.md')).toEqual({
      selected: null,
      selectedPath: null,
      selectionError: null,
    });
    expect(library.getDocument).not.toHaveBeenCalled();
  });

  test('keeps reporting a missing nested document', () => {
    const library = libraryStub();

    expect(resolveLibrarySelection(library, 'missing/index.md')).toEqual({
      selected: null,
      selectedPath: null,
      selectionError: '文档不存在。',
    });
  });
});
