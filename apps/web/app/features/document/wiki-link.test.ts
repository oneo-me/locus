import { parse } from 'comark';
import { describe, expect, test } from 'vitest';
import {
  createWikiLinkDocumentMap,
  createWikiLinkPlugin,
  resolveWikiLink,
  wikiLinkTitlePath,
} from './wiki-link';

describe('wiki link resolution', () => {
  const documents = createWikiLinkDocumentMap([
    { path: 'index.md', title: '首页' },
    { icon: '💬', path: 'operations/index.md', title: '运营' },
    {
      icon: '🎞️',
      path: 'operations/video-workflow/index.md',
      title: '视频工作流',
    },
  ]);

  test('builds a title path from ancestor titles', () => {
    expect(wikiLinkTitlePath('operations/video-workflow', documents)).toBe(
      '运营 / 视频工作流',
    );
    expect(wikiLinkTitlePath('operations', documents)).toBe('运营');
    expect(wikiLinkTitlePath('', documents)).toBe('首页');
  });

  test('resolves an existing article to title, href, icon, and title path', () => {
    expect(resolveWikiLink('operations/video-workflow', documents)).toEqual({
      articlePath: 'operations/video-workflow',
      documentPath: 'operations/video-workflow/index.md',
      href: '/operations/video-workflow',
      icon: '🎞️',
      iconImageModifiedAt: null,
      label: '视频工作流',
      titlePath: '运营 / 视频工作流',
    });
  });

  test('resolves an existing article from an asterweft:// reference', () => {
    expect(
      resolveWikiLink('asterweft://operations/video-workflow', documents),
    ).toEqual({
      articlePath: 'operations/video-workflow',
      documentPath: 'operations/video-workflow/index.md',
      href: '/operations/video-workflow',
      icon: '🎞️',
      iconImageModifiedAt: null,
      label: '视频工作流',
      titlePath: '运营 / 视频工作流',
    });
  });

  test('returns null for missing articles', () => {
    expect(resolveWikiLink('operations/missing', documents)).toBeNull();
    expect(resolveWikiLink('../operations', documents)).toBeNull();
  });

  test('parses wiki links into asterweft-wiki-link nodes', async () => {
    const tree = await parse('见 [[operations/video-workflow]]。', {
      autoUnwrap: false,
      html: false,
      plugins: [createWikiLinkPlugin()],
    });

    expect(tree.nodes).toEqual([
      [
        'p',
        {},
        '见 ',
        ['asterweft-wiki-link', { path: 'operations/video-workflow' }],
        '。',
      ],
    ]);
  });
});
