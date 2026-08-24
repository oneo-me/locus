import { describe, expect, test } from 'vitest';
import {
  documentExportContentWidth,
  documentExportExcludeAttribute,
  documentExportPixelRatio,
  documentImageFilename,
  shouldIncludeDocumentExportNode,
} from './document-export';

describe('document export', () => {
  test('uses the note layout width instead of wide child overflow', () => {
    expect(documentExportContentWidth(832, '0px', '0px')).toBe(832);
    expect(documentExportContentWidth(832, '16px', '16px')).toBe(800);
  });

  test('keeps the canvas within a broadly supported size', () => {
    expect(documentExportPixelRatio(800, 1_000, 2)).toBe(2);
    expect(documentExportPixelRatio(800, 20_000, 2)).toBeLessThan(1);
    expect(documentExportPixelRatio(800, 20_000, 2)).toBeGreaterThan(0.8);
    expect(documentExportPixelRatio(800, 100_000, 2)).toBeLessThan(0.25);
    expect(
      100_000 * documentExportPixelRatio(800, 100_000, 2),
    ).toBeLessThanOrEqual(16_384);
  });

  test('excludes nodes explicitly marked as attachments', () => {
    expect(
      shouldIncludeDocumentExportNode({
        hasAttribute: (name: string) => name === documentExportExcludeAttribute,
      } as unknown as Node),
    ).toBe(false);
    expect(
      shouldIncludeDocumentExportNode({
        hasAttribute: () => false,
      } as unknown as Node),
    ).toBe(true);
  });

  test('creates a safe PNG download filename', () => {
    expect(documentImageFilename(' 路由/验证? ')).toBe('路由-验证-.png');
    expect(documentImageFilename('...')).toBe('asterweft-note.png');
  });
});
