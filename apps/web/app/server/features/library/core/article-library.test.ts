import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, describe, expect, test } from 'vitest';
import { DocumentLibrary } from './document-library.js';

const roots: string[] = [];

function temporaryLibrary(): string {
  const root = mkdtempSync(join(tmpdir(), 'asterweft-articles-'));
  roots.push(root);
  return root;
}

afterEach(() => {
  for (const root of roots.splice(0)) {
    rmSync(root, { recursive: true, force: true });
  }
});

describe('DocumentLibrary articles', () => {
  test('lists direct child articles with child counts', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'guides', 'setup'), { recursive: true });
    writeFileSync(
      join(root, 'guides', 'index.md'),
      '---\ntitle: 指南\n---\n指南正文',
    );
    writeFileSync(
      join(root, 'guides', 'setup', 'index.md'),
      '---\ntitle: 安装\n---\n安装正文',
    );
    const library = await DocumentLibrary.open(root, { watch: false });

    expect(library.listChildArticles()).toEqual([
      expect.objectContaining({
        path: 'guides',
        title: '指南',
        childCount: 1,
      }),
    ]);
    expect(library.listChildArticles('guides')).toEqual([
      expect.objectContaining({
        path: 'guides/setup',
        title: '安装',
        childCount: 0,
      }),
    ]);
    expect(library.listChildArticles()[0]).not.toHaveProperty('hasChildren');
    expect(library.listChildArticles()[0]).not.toHaveProperty('children');
    library.close();
  });

  test('lists articles by path without interpreting Web state from YAML', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'alpha'));
    mkdirSync(join(root, 'zeta'));
    writeFileSync(
      join(root, 'alpha', 'index.md'),
      '---\ntitle: Alpha\norder: 20\n---\n',
    );
    writeFileSync(
      join(root, 'zeta', 'index.md'),
      '---\ntitle: Zeta\norder: 1\n---\n',
    );
    const library = await DocumentLibrary.open(root, { watch: false });

    const articles = library.listChildArticles();
    expect(articles.map((article) => article.path)).toEqual(['alpha', 'zeta']);
    expect(articles[0]).not.toHaveProperty('order');
    library.close();
  });

  test('reads by line and rejects stale metadata updates', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'guide'));
    writeFileSync(
      join(root, 'guide', 'index.md'),
      [
        '---',
        'title: 指南',
        'icon: 📘',
        'order: 2',
        'expanded: true',
        'custom: 保留',
        '---',
        '导语',
        '',
        '# 安装',
        '',
        '安装内容',
        '',
        '# 使用',
        '',
        '使用内容',
      ].join('\n'),
    );
    const library = await DocumentLibrary.open(root, { watch: false });
    const initial = library.readArticleLines({ path: 'guide' });

    expect(initial.content).toContain('# 安装');
    expect(initial.totalLines).toBe(9);
    expect(initial.article.icon).toBe('📘');
    expect(initial.article).not.toHaveProperty('createAt');
    expect(initial.article).not.toHaveProperty('updateAt');
    expect(initial.article).not.toHaveProperty('frontmatter');

    const updated = library.updateArticle({
      path: 'guide',
      expectedRevision: initial.revision,
      metadata: { summary: 'MCP 更新' },
    });
    expect(updated.summary).toBe('MCP 更新');
    const raw = readFileSync(join(root, 'guide', 'index.md'), 'utf8');
    expect(raw).toContain('icon: 📘');
    expect(raw).toContain('summary: MCP 更新');
    expect(raw).not.toContain('order:');
    expect(raw).not.toContain('expanded:');
    expect(raw).not.toContain('custom:');
    expect(raw).toContain('安装内容');

    expect(() =>
      library.updateArticle({
        path: 'guide',
        expectedRevision: initial.revision,
        metadata: { title: '过期更新' },
      }),
    ).toThrow('重新读取');
    library.close();
  });

  test('reads, writes, and edits article bodies with line windows and revisions', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'note'));
    writeFileSync(
      join(root, 'note', 'index.md'),
      '---\ntitle: Note\ncustom: keep\n---\nalpha\nbeta\nbeta\ngamma\n',
    );
    const library = await DocumentLibrary.open(root, { watch: false });

    const window = library.readArticleLines({
      path: 'note',
      offset: 2,
      limit: 2,
    });
    expect(window).toMatchObject({
      offset: 2,
      endLine: 3,
      totalLines: 4,
      content: 'beta\nbeta',
      nextOffset: 4,
    });

    expect(() =>
      library.editArticleContent({
        path: 'note',
        expectedRevision: window.revision,
        oldString: 'beta',
        newString: 'updated',
      }),
    ).toThrow('出现多次');

    const edited = library.editArticleContent({
      path: 'note',
      expectedRevision: window.revision,
      oldString: 'beta',
      newString: 'updated',
      replaceAll: true,
    });
    expect(edited).toMatchObject({
      replacements: 2,
      totalLines: 4,
    });
    expect(edited.revision).not.toBe(window.revision);

    expect(() =>
      library.writeArticleContent({
        path: 'note',
        expectedRevision: window.revision,
        content: 'stale',
      }),
    ).toThrow('重新读取');

    const written = library.writeArticleContent({
      path: 'note',
      expectedRevision: edited.revision,
      content: '# Replaced\n\nbody',
    });
    expect(written).toMatchObject({
      changed: true,
      previousTotalLines: 4,
      totalLines: 3,
    });
    const raw = readFileSync(join(root, 'note', 'index.md'), 'utf8');
    expect(raw).not.toContain('custom: keep');
    expect(raw).toContain('# Replaced\n\nbody');

    const metadataUpdated = library.updateArticle({
      path: 'note',
      expectedRevision: written.revision,
      metadata: {
        title: 'Updated note',
        icon: '🧭',
        summary: 'Metadata only',
      },
    });
    expect(metadataUpdated).toMatchObject({
      title: 'Updated note',
      icon: '🧭',
      summary: 'Metadata only',
    });
    expect(metadataUpdated).not.toHaveProperty('updateAt');
    const metadataRaw = readFileSync(join(root, 'note', 'index.md'), 'utf8');
    expect(metadataRaw).toContain('summary: Metadata only');
    expect(metadataRaw).not.toContain('custom:');
    expect(metadataRaw).toContain('# Replaced\n\nbody');

    const metadataRevision = library.readArticleLines({
      path: 'note',
      limit: 1,
    }).revision;
    expect(() =>
      library.updateArticle({
        path: 'note',
        expectedRevision: metadataRevision,
        metadata: {},
      }),
    ).toThrow('至少需要提供一个元数据更新');
    library.close();
  });

  test('requires parent articles and supports recursive article deletion', async () => {
    const root = temporaryLibrary();
    const library = await DocumentLibrary.open(root, { watch: false });

    expect(() =>
      library.createArticle({
        path: 'guides/setup',
        title: '安装',
      }),
    ).toThrow('文章不存在');

    library.createArticle({ path: 'guides', title: '指南' });
    library.createArticle({
      path: 'guides/setup',
      title: '安装',
      content: '# 安装',
    });
    writeFileSync(join(root, 'guides', 'attachment.pdf'), '附件');
    expect(() => library.deleteArticle({ path: '', recursive: true })).toThrow(
      '虚拟根文章',
    );
    expect(existsSync(join(root, 'guides', 'index.md'))).toBe(true);
    expect(() => library.deleteArticle({ path: 'guides' })).toThrow(
      'recursive',
    );

    expect(library.deleteArticle({ path: 'guides', recursive: true })).toEqual({
      path: 'guides',
      recursive: true,
      deletedArticles: 2,
    });
    expect(library.listChildArticles()).toEqual([]);
    library.close();
  });

  test('moves article paths while preserving titles and the subtree', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'guide', 'setup'), { recursive: true });
    writeFileSync(
      join(root, 'guide', 'index.md'),
      '---\ntitle: 指南\n---\n指南正文',
    );
    writeFileSync(
      join(root, 'guide', 'setup', 'index.md'),
      '---\ntitle: 安装\n---\n安装正文',
    );
    writeFileSync(join(root, 'guide', 'diagram.svg'), '<svg></svg>');
    const library = await DocumentLibrary.open(root, { watch: false });
    const initial = library.readArticleLines({ path: 'guide', limit: 1 });

    expect(
      library.moveArticle({
        path: 'guide',
        newPath: 'handbook',
        expectedRevision: initial.revision,
      }),
    ).toEqual({
      previousPath: 'guide',
      path: 'handbook',
      movedArticles: 2,
    });
    expect(() => library.readArticleLines({ path: 'guide', limit: 1 })).toThrow(
      '文章不存在',
    );
    expect(
      library.readArticleLines({ path: 'handbook', limit: 1 }).article.title,
    ).toBe('指南');
    expect(
      library.readArticleLines({ path: 'handbook/setup', limit: 1 }).article
        .title,
    ).toBe('安装');
    expect(library.listArticleAttachments('handbook')).toEqual([
      expect.objectContaining({ name: 'diagram.svg' }),
    ]);
    expect(existsSync(join(root, 'guide'))).toBe(false);
    expect(readFileSync(join(root, 'handbook', 'diagram.svg'), 'utf8')).toBe(
      '<svg></svg>',
    );
    library.close();
  });

  test('rejects stale, colliding, and self-descendant article moves', async () => {
    const root = temporaryLibrary();
    const library = await DocumentLibrary.open(root, { watch: false });
    library.createArticle({ path: 'guide', title: '指南' });
    library.createArticle({ path: 'other', title: '其它' });
    const initial = library.readArticleLines({ path: 'guide', limit: 1 });
    library.updateArticle({
      path: 'guide',
      expectedRevision: initial.revision,
      metadata: { summary: '已更新' },
    });

    expect(() =>
      library.moveArticle({
        path: 'guide',
        newPath: 'handbook',
        expectedRevision: initial.revision,
      }),
    ).toThrow('重新读取');
    const latest = library.readArticleLines({ path: 'guide', limit: 1 });
    expect(() =>
      library.moveArticle({
        path: 'guide',
        newPath: 'other',
        expectedRevision: latest.revision,
      }),
    ).toThrow('已经存在');
    expect(() =>
      library.moveArticle({
        path: 'guide',
        newPath: 'guide/child',
        expectedRevision: latest.revision,
      }),
    ).toThrow('自己的子文章路径');
    library.close();
  });

  test('lists, uploads, downloads, overwrites, and deletes attachments safely', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'guide'));
    writeFileSync(
      join(root, 'guide', 'index.md'),
      '---\ntitle: 指南\n---\n正文',
    );
    writeFileSync(join(root, 'index.md'), '---\ntitle: 根文章\n---\n正文');
    writeFileSync(join(root, 'guide', 'existing.bin'), '旧附件');
    const library = await DocumentLibrary.open(root, { watch: false });

    expect(library.listArticleAttachments('guide')).toEqual([
      expect.objectContaining({
        name: 'existing.bin',
        size: new TextEncoder().encode('旧附件').byteLength,
      }),
    ]);

    const uploaded = library.writeArticleAttachment({
      path: 'guide',
      name: '说明.txt',
      data: new TextEncoder().encode('abcdef'),
    });
    expect(uploaded).toMatchObject({ name: '说明.txt', size: 6 });
    expect(() =>
      library.writeArticleAttachment({
        path: 'guide',
        name: '说明.txt',
        data: new TextEncoder().encode('重复'),
      }),
    ).toThrow('已经存在');

    const firstChunk = library.readArticleAttachment({
      path: 'guide',
      name: '说明.txt',
      maxBytes: 3,
    });
    expect(new TextDecoder().decode(firstChunk.data)).toBe('abc');
    expect(firstChunk.nextOffset).toBe(3);
    const secondChunk = library.readArticleAttachment({
      path: 'guide',
      name: '说明.txt',
      offset: firstChunk.nextOffset ?? 0,
      maxBytes: 3,
    });
    expect(new TextDecoder().decode(secondChunk.data)).toBe('def');
    expect(secondChunk.nextOffset).toBeNull();

    library.writeArticleAttachment({
      path: 'guide',
      name: '说明.txt',
      data: new TextEncoder().encode('已覆盖'),
      overwrite: true,
    });
    expect(readFileSync(join(root, 'guide', '说明.txt'), 'utf8')).toBe(
      '已覆盖',
    );

    expect(() =>
      library.writeArticleAttachment({
        path: 'guide',
        name: 'index.md',
        data: new Uint8Array(),
        overwrite: true,
      }),
    ).toThrow('文章正文文件');
    expect(() =>
      library.writeArticleAttachment({
        path: '',
        name: 'cache.sqlite',
        data: new Uint8Array(),
        overwrite: true,
      }),
    ).toThrow('根文章不存在');

    expect(
      library.deleteArticleAttachment({
        path: 'guide',
        name: '说明.txt',
      }),
    ).toEqual({
      path: 'guide',
      name: '说明.txt',
      deleted: true,
    });
    expect(library.listArticleAttachments('guide')).toEqual([
      expect.objectContaining({ name: 'existing.bin' }),
    ]);
    library.close();
  });

  test('updates only affected cache entries for MCP write operations', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'guide'));
    mkdirSync(join(root, 'unrelated'));
    writeFileSync(
      join(root, 'guide', 'index.md'),
      '---\ntitle: 指南\n---\n正文',
    );
    writeFileSync(
      join(root, 'unrelated', 'index.md'),
      '---\ntitle: 原始标题\n---\n正文',
    );
    const library = await DocumentLibrary.open(root, { watch: false });

    writeFileSync(
      join(root, 'unrelated', 'index.md'),
      '---\ntitle: 外部更新\n---\n正文',
    );
    library.writeArticleAttachment({
      path: 'guide',
      name: 'sample.txt',
      data: new TextEncoder().encode('sample'),
    });

    expect(library.getDocument('unrelated/index.md').title).toBe('原始标题');
    expect(library.listArticleAttachments('guide')).toEqual([
      expect.objectContaining({ name: 'sample.txt', size: 6 }),
    ]);

    library.deleteArticleAttachment({
      path: 'guide',
      name: 'sample.txt',
    });
    expect(library.getDocument('unrelated/index.md').title).toBe('原始标题');

    await library.refresh();
    expect(library.getDocument('unrelated/index.md').title).toBe('外部更新');
    library.close();
  });
});
