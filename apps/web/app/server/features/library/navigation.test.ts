import { describe, expect, test } from 'vitest';
import type { DocumentSummary } from './core/types';
import { toLibraryNavigationDocument } from './navigation';

const summary: DocumentSummary = {
  assets: ['report.pdf'],
  directory: 'guide',
  icon: '📘',
  iconImageModifiedAt: null,
  issue: null,
  metadataKeys: ['title', 'summary'],
  modifiedAt: '2026-08-02T00:00:00.000Z',
  path: 'guide/index.md',
  size: 1_024,
  summary: 'Only the article view needs this.',
  title: 'Guide',
  wordCount: 120,
};

describe('library navigation projection', () => {
  test('keeps only fields required by the document tree', () => {
    expect(toLibraryNavigationDocument(summary)).toEqual({
      attachmentCount: 1,
      directory: 'guide',
      icon: '📘',
      iconImageModifiedAt: null,
      path: 'guide/index.md',
      title: 'Guide',
    });
  });
});
