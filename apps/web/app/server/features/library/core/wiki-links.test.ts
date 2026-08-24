import { describe, expect, test } from 'vitest';
import {
  extractWikiLinkArticlePaths,
  extractWikiLinkDocumentPaths,
  parseWikiLinkArticlePath,
} from './wiki-links.js';

describe('parseWikiLinkArticlePath', () => {
  test('accepts library-absolute article paths', () => {
    expect(parseWikiLinkArticlePath('operations/video-workflow')).toBe(
      'operations/video-workflow',
    );
    expect(parseWikiLinkArticlePath('  operations/video-workflow  ')).toBe(
      'operations/video-workflow',
    );
    expect(parseWikiLinkArticlePath('operations/video-workflow/index.md')).toBe(
      'operations/video-workflow',
    );
    expect(parseWikiLinkArticlePath('index.md')).toBe('');
  });

  test('accepts asterweft:// reference targets', () => {
    expect(
      parseWikiLinkArticlePath('asterweft://operations/video-workflow'),
    ).toBe('operations/video-workflow');
    expect(parseWikiLinkArticlePath('asterweft://index.md')).toBe('');
  });

  test('rejects empty, relative, alias, and unsafe targets', () => {
    expect(parseWikiLinkArticlePath('')).toBeNull();
    expect(parseWikiLinkArticlePath('   ')).toBeNull();
    expect(parseWikiLinkArticlePath('../operations')).toBeNull();
    expect(parseWikiLinkArticlePath('./sibling')).toBeNull();
    expect(
      parseWikiLinkArticlePath('operations/video-workflow|别名'),
    ).toBeNull();
    expect(parseWikiLinkArticlePath('/absolute')).toBeNull();
  });
});

describe('extractWikiLinkArticlePaths', () => {
  test('extracts wiki targets in order and keeps duplicates', () => {
    expect(
      extractWikiLinkArticlePaths(
        '见 [[operations/video-workflow]] 与 [[guide]]，再提 [[guide]]。',
      ),
    ).toEqual(['operations/video-workflow', 'guide', 'guide']);
  });

  test('skips fenced and inline code', () => {
    expect(
      extractWikiLinkArticlePaths(
        [
          '正文 [[guide]]',
          '```',
          '[[operations/video-workflow]]',
          '```',
          '行内 `[[architecture]]` 结束',
          '~~~md',
          '[[notes]]',
          '~~~',
        ].join('\n'),
      ),
    ).toEqual(['guide']);
  });

  test('maps extracted targets to document paths', () => {
    expect(
      extractWikiLinkDocumentPaths(
        '[[operations/video-workflow]] [[index.md]]',
      ),
    ).toEqual(['operations/video-workflow/index.md', 'index.md']);
  });
});
