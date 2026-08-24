import type { LibraryNavigationDocument } from '~/contracts/library-api';
import type { LibraryRepositoryChange } from '~/contracts/repository';
import { describe, expect, test } from 'vitest';
import { buildDocumentTree } from '~/features/library/tree';
import {
  getDocumentChangeStatuses,
  getDocumentTreeChangeLabel,
  getDocumentTreeChangeStates,
} from './document-change-status';

function document(path: string): LibraryNavigationDocument {
  return {
    attachmentCount: 0,
    path,
    directory: path.slice(0, path.lastIndexOf('/')),
    title: path,
    icon: null,
    iconImageModifiedAt: null,
  };
}

function change(
  input: Pick<LibraryRepositoryChange, 'file' | 'status'> &
    Partial<LibraryRepositoryChange>,
): LibraryRepositoryChange {
  return {
    staged: false,
    unstaged: true,
    ...input,
  };
}

describe('document repository change statuses', () => {
  test('marks new articles green and existing article edits yellow', () => {
    const statuses = getDocumentChangeStatuses(
      [document('notes/index.md'), document('new/index.md')],
      [
        change({ file: 'notes/index.md', status: 'modified' }),
        change({ file: 'new/index.md', status: 'added' }),
      ],
    );

    expect([...statuses]).toEqual([
      ['notes/index.md', 'modified'],
      ['new/index.md', 'added'],
    ]);
  });

  test('marks attachment changes as article modifications', () => {
    const statuses = getDocumentChangeStatuses(
      [document('notes/index.md'), document('notes/child/index.md')],
      [
        change({ file: 'notes/photo.png', status: 'added' }),
        change({
          file: 'notes/old.pdf',
          status: 'deleted',
        }),
      ],
    );

    expect([...statuses]).toEqual([['notes/index.md', 'modified']]);
  });

  test('propagates added and modified states through every ancestor', () => {
    const documents = [
      document('notes/index.md'),
      document('notes/child/index.md'),
      document('notes/child/deep/index.md'),
      document('notes/new/index.md'),
    ];
    const tree = buildDocumentTree(documents, [
      'notes',
      'notes/child',
      'notes/child/deep',
      'notes/new',
    ]);
    const documentStatuses = getDocumentChangeStatuses(documents, [
      change({ file: 'notes/child/deep/index.md', status: 'modified' }),
      change({ file: 'notes/new/index.md', status: 'added' }),
    ]);

    const states = getDocumentTreeChangeStates(tree, documentStatuses);

    expect(states.get('notes')).toEqual({
      own: null,
      descendants: ['added', 'modified'],
    });
    expect(states.get('notes/child')).toEqual({
      own: null,
      descendants: ['modified'],
    });
    expect(states.get('notes/child/deep')).toEqual({
      own: 'modified',
      descendants: [],
    });
    expect(states.get('notes/new')).toEqual({
      own: 'added',
      descendants: [],
    });
  });

  test('keeps current and descendant changes independently visible', () => {
    const documents = [
      document('notes/index.md'),
      document('notes/child/index.md'),
    ];
    const tree = buildDocumentTree(documents, ['notes', 'notes/child']);
    const documentStatuses = getDocumentChangeStatuses(documents, [
      change({ file: 'notes/index.md', status: 'modified' }),
      change({ file: 'notes/child/index.md', status: 'added' }),
    ]);

    const state = getDocumentTreeChangeStates(tree, documentStatuses).get(
      'notes',
    );

    expect(state).toBeDefined();
    if (!state) throw new Error('Expected notes to have a change state.');
    expect(state).toEqual({
      own: 'modified',
      descendants: ['added'],
    });
    expect(getDocumentTreeChangeLabel(state)).toBe('已修改，尚未提交');
  });

  test('describes mixed descendant changes behind one inherited state', () => {
    expect(
      getDocumentTreeChangeLabel({
        own: null,
        descendants: ['added', 'modified'],
      }),
    ).toBe('子文章有新增和修改，尚未提交');
  });
});
