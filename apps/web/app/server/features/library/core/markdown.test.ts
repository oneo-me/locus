import { describe, expect, test } from 'vitest';
import { parseDocument, serializeDocument } from './markdown.js';

describe('parseDocument summary', () => {
  test('records declared summary', () => {
    const now = new Date('2026-07-24T00:00:00.000Z');
    const parsed = parseDocument(
      'notes/index.md',
      'notes',
      ['---', 'title: 笔记', 'summary: 一句话摘要', '---', '正文'].join('\n'),
      { mtime: now, size: 10 },
    );

    expect(parsed.summary.metadataKeys).toEqual(['title', 'summary']);
    expect(parsed.summary.summary).toBe('一句话摘要');
  });

  test('serializes only title icon and summary', () => {
    const yaml = serializeDocument(
      {
        title: '笔记',
        icon: '📘',
        summary: '一句话摘要',
      },
      '正文',
    );
    expect(yaml).toContain('title: 笔记');
    expect(yaml).toContain('icon: 📘');
    expect(yaml).toContain('summary: 一句话摘要');
    expect(yaml).not.toContain('description:');
    expect(yaml).not.toContain('source:');
    expect(yaml).not.toContain('createAt:');
    expect(yaml).not.toContain('updateAt:');
    expect(yaml).not.toContain('assets:');
    expect(yaml).not.toContain('tags:');
  });

  test('omits null optional fields', () => {
    const yaml = serializeDocument(
      {
        title: '笔记',
        icon: null,
        summary: null,
      },
      '正文',
    );
    expect(yaml).toContain('title: 笔记');
    expect(yaml).not.toContain('icon:');
    expect(yaml).not.toContain('summary:');
  });

  test('ignores unknown frontmatter keys', () => {
    const now = new Date('2026-07-24T00:00:00.000Z');
    const parsed = parseDocument(
      'notes/index.md',
      'notes',
      [
        '---',
        'title: 笔记',
        'summary: 摘要',
        'description: 旧描述',
        'source: https://example.com',
        'createAt: 2020-01-01T00:00:00.000Z',
        'updateAt: 2020-01-02T00:00:00.000Z',
        'assets: [a.pdf]',
        'tags: [a]',
        'order: 3',
        'custom: 额外',
        '---',
        '正文',
      ].join('\n'),
      { mtime: now, size: 10 },
    );

    expect(parsed.summary.metadataKeys).toEqual(['title', 'summary']);
    expect(parsed.summary.summary).toBe('摘要');
    expect(parsed.summary).not.toHaveProperty('createAt');
    expect(parsed.summary).not.toHaveProperty('updateAt');
  });
});
