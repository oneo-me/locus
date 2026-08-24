import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';
import { afterEach, describe, expect, test } from 'vitest';
import { DocumentCache } from './document-cache.js';
import { DocumentLibrary } from './document-library.js';
import { openSqliteDatabase } from './sqlite-runtime.js';

const roots: string[] = [];

function temporaryLibrary(): string {
  const root = mkdtempSync(join(tmpdir(), 'asterweft-core-'));
  roots.push(root);
  return root;
}

function pngIcon(size = 64): Uint8Array {
  const data = new Uint8Array(45);
  data.set([137, 80, 78, 71, 13, 10, 26, 10]);
  const view = new DataView(data.buffer);
  view.setUint32(8, 13);
  data.set([73, 72, 68, 82], 12);
  view.setUint32(16, size);
  view.setUint32(20, size);
  data.set([73, 69, 78, 68], 37);
  return data;
}

async function waitFor(assertion: () => void, timeout: number): Promise<void> {
  const deadline = Date.now() + timeout;
  let lastError: unknown;
  do {
    try {
      assertion();
      return;
    } catch (error) {
      lastError = error;
      await sleep(25);
    }
  } while (Date.now() < deadline);
  throw lastError;
}

afterEach(() => {
  for (const root of roots.splice(0)) {
    rmSync(root, { recursive: true, force: true });
  }
});

describe('DocumentLibrary', () => {
  test('removes the legacy heading index from an existing cache', () => {
    const root = temporaryLibrary();
    const cachePath = join(root, 'cache.sqlite');
    const cache = new DocumentCache(cachePath);
    cache.close();

    const legacyDatabase = openSqliteDatabase(cachePath);
    legacyDatabase.exec(
      "ALTER TABLE documents ADD COLUMN headings TEXT NOT NULL DEFAULT '[]'",
    );
    legacyDatabase.close();

    const migratedCache = new DocumentCache(cachePath);
    migratedCache.close();
    const migratedDatabase = openSqliteDatabase(cachePath);
    const columns = migratedDatabase
      .prepare('PRAGMA table_info(documents)')
      .all() as Array<{ name: string }>;
    migratedDatabase.close();

    expect(columns.some((column) => column.name === 'headings')).toBe(false);
  });

  test('indexes only nested index.md documents and ignores the library root index', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'guide', 'setup'), { recursive: true });
    mkdirSync(join(root, 'guide', 'assets'));
    mkdirSync(join(root, 'empty'));
    writeFileSync(
      join(root, 'index.md'),
      '---\ntitle: 首页\n---\n文档库首页。',
    );
    writeFileSync(
      join(root, 'guide', 'index.md'),
      '---\ntitle: 开始\nsummary: 第一步\n---\n# 安装\n\n运行 Bun。',
    );
    writeFileSync(
      join(root, 'guide', 'setup', 'index.md'),
      '---\ntitle: 配置\n---\n配置项目。',
    );
    writeFileSync(
      join(root, 'guide', 'assets', 'index.md'),
      '---\ntitle: 附件说明\n---\nassets 目录不再保留。',
    );
    writeFileSync(
      join(root, 'guide', 'notes.md'),
      '---\ntitle: 普通 Markdown\n---\n不应被索引的内容。',
    );
    writeFileSync(join(root, 'guide', 'setup.pdf'), 'sibling attachment');
    writeFileSync(join(root, 'guide', 'assets', 'diagram.svg'), '<svg></svg>');
    const library = await DocumentLibrary.open(root, { watch: false });

    expect(library.status()).toMatchObject({
      documentCount: 3,
      watching: false,
    });
    expect(() => library.getDocument('index.md')).toThrow('文档不存在');
    expect(library.searchDocuments('文档库首页').total).toBe(0);
    expect(library.listDocuments('guide').documents[0]?.title).toBe('开始');
    expect(library.getDocument('guide/index.md').metadataKeys).toEqual([
      'title',
      'summary',
    ]);
    expect(library.getDocument('guide/setup/index.md').title).toBe('配置');
    expect(library.listDocuments('guide').directories).toEqual([
      'guide/assets',
      'guide/setup',
    ]);
    expect(library.listAllDirectories()).toEqual([
      'guide',
      'guide/assets',
      'guide/setup',
    ]);
    expect(library.searchDocuments('Bun').results[0]?.path).toBe(
      'guide/index.md',
    );
    expect(library.searchDocuments('不应被索引的内容').total).toBe(0);
    expect(library.getDocument('guide/index.md').assets).toEqual([
      'notes.md',
      'setup.pdf',
    ]);
    expect(
      library
        .getDocumentGraph()
        .nodes.filter((node) => node.kind === 'attachment')
        .map((node) => node.title)
        .sort(),
    ).toEqual(['diagram.svg', 'notes.md', 'setup.pdf']);
    const siblingAsset = library.readAsset('guide/index.md', 'setup.pdf');
    expect(new TextDecoder().decode(siblingAsset.data)).toBe(
      'sibling attachment',
    );
    const nestedAsset = library.readAsset(
      'guide/assets/index.md',
      'diagram.svg',
    );
    expect(nestedAsset.name).toBe('diagram.svg');
    expect(nestedAsset.size).toBeGreaterThan(0);
    expect(new TextDecoder().decode(nestedAsset.data)).toBe('<svg></svg>');
    expect(() => library.readAsset('guide/index.md', 'unlisted.svg')).toThrow(
      '文件不是该文档的附件',
    );
    expect(() => library.readAsset('guide/index.md', 'missing.pdf')).toThrow(
      '文件不是该文档的附件',
    );
    expect(statSync(join(root, 'cache.sqlite')).size).toBeGreaterThan(0);
    library.close();
  });

  test('stores cache outside the document library when cachePath is provided', async () => {
    const root = temporaryLibrary();
    const cacheRoot = temporaryLibrary();
    const cachePath = join(cacheRoot, 'nested', 'library.sqlite');
    mkdirSync(join(root, 'notes'), { recursive: true });
    writeFileSync(
      join(root, 'notes', 'index.md'),
      '---\ntitle: 笔记\n---\n正文',
    );

    const library = await DocumentLibrary.open(root, {
      cachePath,
      watch: false,
    });

    expect(library.status().cachePath).toBe(cachePath);
    expect(statSync(cachePath).size).toBeGreaterThan(0);
    expect(() => statSync(join(root, 'cache.sqlite'))).toThrow();
    expect(library.getDocument('notes/index.md').title).toBe('笔记');
    library.close();
  });

  test('writes only title icon and summary into frontmatter', async () => {
    const root = temporaryLibrary();
    const library = await DocumentLibrary.open(root, { watch: false });
    const created = library.createDocument({
      path: 'notes/new/index.md',
      title: '新文档',
      content: '# 正文',
    });
    writeFileSync(join(root, 'notes', 'new', 'brief.pdf'), 'attachment');
    await library.refresh();
    const updated = library.updateDocument({
      path: 'notes/new/index.md',
      summary: '更新后的摘要',
    });

    expect(created.metadataKeys).toEqual(['title']);
    expect(updated).toMatchObject({
      title: '新文档',
      summary: '更新后的摘要',
      assets: ['brief.pdf'],
      content: '# 正文',
      metadataKeys: ['title', 'summary'],
    });
    const bodyUpdated = library.updateDocument({
      path: 'notes/new/index.md',
      content: '# 新正文',
    });
    expect(bodyUpdated).toMatchObject({
      content: '# 新正文',
    });
    const writtenPath = join(root, 'notes', 'new', 'index.md');
    expect(statSync(writtenPath).size).toBeGreaterThan(0);
    const writtenDocument = readFileSync(writtenPath, 'utf8');
    expect(writtenDocument).toContain('title: 新文档');
    expect(writtenDocument).toContain('summary: 更新后的摘要');
    expect(writtenDocument).not.toContain('assets:');
    expect(writtenDocument).not.toContain('createAt:');
    expect(writtenDocument).not.toContain('updateAt:');
    expect(writtenDocument).not.toContain('description:');
    expect(writtenDocument).not.toContain('source:');
    library.close();
  });

  test('parses and updates icon metadata', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'notes'));
    writeFileSync(
      join(root, 'notes', 'index.md'),
      '---\ntitle: 笔记\nicon: 📘\n---\n正文',
    );
    const library = await DocumentLibrary.open(root, { watch: false });

    expect(library.getDocument('notes/index.md')).toMatchObject({
      icon: '📘',
      metadataKeys: ['title', 'icon'],
    });

    const updated = library.updateDocument({
      path: 'notes/index.md',
      icon: ' 📗 ',
    });
    expect(updated).toMatchObject({ icon: '📗' });
    const written = readFileSync(join(root, 'notes', 'index.md'), 'utf8');
    expect(written).toContain('icon: 📗');

    const cleared = library.updateDocument({
      path: 'notes/index.md',
      icon: null,
    });
    expect(cleared).toMatchObject({ icon: null });
    library.close();
  });

  test('stores a protected 64px PNG beside index.md without exposing it as an attachment', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'notes'));
    writeFileSync(
      join(root, 'notes', 'index.md'),
      '---\ntitle: 笔记\n---\n正文',
    );
    writeFileSync(join(root, 'notes', 'brief.pdf'), 'attachment');
    writeFileSync(join(root, 'notes', 'icon.png'), pngIcon());
    const library = await DocumentLibrary.open(root, { watch: false });

    expect(library.getDocument('notes/index.md')).toMatchObject({
      assets: ['brief.pdf'],
      iconImageModifiedAt: expect.any(String),
    });
    expect(library.readArticleLines({ path: 'notes' }).article).toMatchObject({
      icon: 'icon.png',
    });
    expect(library.readDocumentIcon('notes/index.md')).toMatchObject({
      name: 'icon.png',
      size: 45,
    });
    expect(library.listArticleAttachments('notes')).toEqual([
      expect.objectContaining({ name: 'brief.pdf' }),
    ]);
    expect(() =>
      library.writeArticleAttachment({
        path: 'notes',
        name: 'ICON.PNG',
        data: pngIcon(),
      }),
    ).toThrow('受保护的 icon.png');
    expect(() =>
      library.deleteArticleAttachment({
        path: 'notes',
        name: 'icon.png',
      }),
    ).toThrow('受保护的 icon.png');
    expect(() =>
      library.writeDocumentIcon('notes/index.md', pngIcon(32)),
    ).toThrow('64×64');

    library.writeDocumentIcon('notes/index.md', pngIcon());
    expect(library.readArticleLines({ path: 'notes' }).article.icon).toBe(
      'icon.png',
    );
    expect([...readFileSync(join(root, 'notes', 'icon.png'))]).toEqual([
      ...pngIcon(),
    ]);
    expect(
      library.deleteDocumentIcon('notes/index.md').iconImageModifiedAt,
    ).toBeNull();
    expect(() => statSync(join(root, 'notes', 'icon.png'))).toThrow();
    library.close();
  });

  test('falls back to the directory name when frontmatter has no title', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'legacy'));
    writeFileSync(
      join(root, 'legacy', 'index.md'),
      '---\nsummary: 旧文档\n---\n正文',
    );
    const library = await DocumentLibrary.open(root, { watch: false });

    expect(library.getDocument('legacy/index.md')).toMatchObject({
      title: 'legacy',
      summary: '旧文档',
      metadataKeys: ['summary'],
    });
    library.close();
  });

  test('observes external file changes', async () => {
    const watchTimeout = 10_000;
    const root = temporaryLibrary();
    const library = await DocumentLibrary.open(root, {
      watch: true,
      watchDebounceMs: 20,
      watchIntervalMs: 100,
      watchUsePolling: true,
    });
    mkdirSync(join(root, 'external'));
    writeFileSync(
      join(root, 'external', 'index.md'),
      '---\ntitle: 外部文档\n---\n变化',
    );

    await waitFor(
      () => expect(library.status().documentCount).toBe(1),
      watchTimeout,
    );
    expect(library.getDocument('external/index.md').title).toBe('外部文档');

    writeFileSync(join(root, 'external', 'diagram.png'), 'image');
    await waitFor(
      () =>
        expect(library.getDocument('external/index.md').assets).toEqual([
          'diagram.png',
        ]),
      watchTimeout,
    );

    rmSync(join(root, 'external', 'diagram.png'));
    await waitFor(
      () => expect(library.getDocument('external/index.md').assets).toEqual([]),
      watchTimeout,
    );

    writeFileSync(join(root, 'external', 'icon.png'), pngIcon());
    await waitFor(
      () =>
        expect(
          library.getDocument('external/index.md').iconImageModifiedAt,
        ).not.toBeNull(),
      watchTimeout,
    );
    expect(library.getDocument('external/index.md').assets).toEqual([]);

    rmSync(join(root, 'external'), { recursive: true });
    await waitFor(
      () => expect(library.listAllDirectories()).toEqual([]),
      watchTimeout,
    );
    library.close();
  }, 35_000);

  test('reports the external refresh lifecycle', async () => {
    const root = temporaryLibrary();
    const library = await DocumentLibrary.open(root, {
      watch: true,
      watchDebounceMs: 40,
      watchIntervalMs: 20,
      watchMaxWaitMs: 200,
      watchUsePolling: true,
    });
    const refreshStates: boolean[] = [];
    const unsubscribe = library.subscribe((status) => {
      refreshStates.push(status.refreshing);
    });

    mkdirSync(join(root, 'external'));
    writeFileSync(
      join(root, 'external', 'index.md'),
      '---\ntitle: 外部文档\n---\n正文',
    );

    await waitFor(() => expect(library.status().documentCount).toBe(1), 5_000);
    expect(refreshStates).toContain(true);
    expect(refreshStates.at(-1)).toBe(false);

    unsubscribe();
    library.close();
  });

  test('refreshes a continuing event batch at the maximum wait', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'notes'));
    const documentPath = join(root, 'notes', 'index.md');
    writeFileSync(documentPath, '---\ntitle: 初始\n---\n正文');
    const library = await DocumentLibrary.open(root, {
      watch: true,
      watchDebounceMs: 200,
      watchIntervalMs: 20,
      watchMaxWaitMs: 350,
      watchUsePolling: true,
    });

    const writes = (async () => {
      for (let index = 1; index <= 15; index += 1) {
        writeFileSync(documentPath, `---\ntitle: 更新 ${index}\n---\n正文`);
        await sleep(50);
      }
    })();

    await sleep(550);
    expect(library.getDocument('notes/index.md').title).not.toBe('初始');

    await writes;
    await waitFor(
      () => expect(library.getDocument('notes/index.md').title).toBe('更新 15'),
      5_000,
    );
    library.close();
  });

  test('does not emit another change for an unchanged refresh', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'notes'));
    writeFileSync(
      join(root, 'notes', 'index.md'),
      '---\ntitle: 笔记\n---\n正文',
    );
    const library = await DocumentLibrary.open(root, { watch: false });
    let notifications = 0;
    const unsubscribe = library.subscribe(() => {
      notifications += 1;
    });

    library.updateDocument({
      path: 'notes/index.md',
      content: '更新后的正文',
    });
    expect(notifications).toBe(1);

    await library.refresh();
    expect(notifications).toBe(1);

    unsubscribe();
    library.close();
  });

  test('does not advance the index when the watcher observes an internal write', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'notes'));
    writeFileSync(
      join(root, 'notes', 'index.md'),
      '---\ntitle: 笔记\n---\n正文',
    );
    const library = await DocumentLibrary.open(root, {
      watch: true,
      watchDebounceMs: 20,
      watchIntervalMs: 100,
      watchUsePolling: true,
    });
    const statuses: Array<{ indexedAt: string | null; refreshing: boolean }> =
      [];
    const unsubscribe = library.subscribe((status) => {
      statuses.push({
        indexedAt: status.indexedAt,
        refreshing: status.refreshing,
      });
    });

    library.updateDocument({
      path: 'notes/index.md',
      content: '更新后的正文',
    });
    await new Promise((resolve) => setTimeout(resolve, 400));

    expect(statuses.map((status) => status.refreshing)).toEqual([
      false,
      true,
      false,
    ]);
    expect(new Set(statuses.map((status) => status.indexedAt)).size).toBe(1);
    unsubscribe();
    library.close();
  });

  test('reuses the document graph until indexed content changes', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'notes'));
    writeFileSync(
      join(root, 'notes', 'index.md'),
      '---\ntitle: 笔记\n---\n正文',
    );
    const library = await DocumentLibrary.open(root, { watch: false });

    const initial = library.getDocumentGraph();
    expect(library.getDocumentGraph()).toBe(initial);

    library.updateDocument({
      path: 'notes/index.md',
      content: '[链接](../other)',
    });
    expect(library.getDocumentGraph()).not.toBe(initial);
    library.close();
  });

  test('builds and caches a graph without excluded documents', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'included'));
    mkdirSync(join(root, 'excluded'));
    writeFileSync(
      join(root, 'included', 'index.md'),
      '---\ntitle: 保留\n---\n[隐藏文章](../excluded)',
    );
    writeFileSync(
      join(root, 'excluded', 'index.md'),
      '---\ntitle: 隐藏\n---\n正文',
    );
    const library = await DocumentLibrary.open(root, { watch: false });

    const graph = library.getDocumentGraph(['excluded/index.md']);
    expect(graph.nodes.map((node) => node.id)).toEqual(['included/index.md']);
    expect(graph.edges).toHaveLength(0);
    expect(library.getDocumentGraph(['excluded/index.md'])).toBe(graph);
    expect(library.getDocumentGraph()).not.toBe(graph);
    library.close();
  });

  test('drops unknown frontmatter keys when updating metadata', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'notes'));
    writeFileSync(
      join(root, 'notes', 'index.md'),
      '---\ntitle: 笔记\ncustom: 保留\ndraft: true\n---\n正文',
    );
    const library = await DocumentLibrary.open(root, { watch: false });

    library.updateDocument({
      path: 'notes/index.md',
      icon: '📘',
    });

    const raw = readFileSync(join(root, 'notes', 'index.md'), 'utf8');
    expect(raw).toContain('title: 笔记');
    expect(raw).toContain('icon: 📘');
    expect(raw).not.toContain('custom:');
    expect(raw).not.toContain('draft:');
    library.close();
  });

  test('rejects traversal and non-index document paths', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'safe'), { recursive: true });
    writeFileSync(
      join(root, 'safe', 'index.md'),
      '---\ntitle: 安全\n---\n正文',
    );
    writeFileSync(join(root, 'secret.txt'), 'secret');
    const library = await DocumentLibrary.open(root, { watch: false });
    expect(library.getDocument('safe/index.md').metadataKeys).toEqual([
      'title',
    ]);
    expect(library.getDocument('safe/index.md').assets).toEqual([]);
    expect(() => library.createDocument({ path: '../index.md' })).toThrow();
    expect(() => library.createDocument({ path: 'notes/new.md' })).toThrow();
    expect(() => library.readAsset('safe/index.md', '../secret.txt')).toThrow();
    library.close();
  });
});
