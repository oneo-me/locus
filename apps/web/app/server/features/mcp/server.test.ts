import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  Client,
  StreamableHTTPClientTransport,
} from '@modelcontextprotocol/client';
import { createMcpHandler } from '@modelcontextprotocol/server';
import { DocumentLibrary } from '~/server/features/library/core/document-library';
import { execFileSync } from '~/server/test/node-process';
import { afterEach, describe, expect, test } from 'vitest';
import { createAsterweftMcpServer } from './server';

const roots: string[] = [];

function temporaryLibrary(): string {
  const root = mkdtempSync(join(tmpdir(), 'asterweft-mcp-'));
  roots.push(root);
  return root;
}

afterEach(() => {
  for (const root of roots.splice(0)) {
    rmSync(root, { recursive: true, force: true });
  }
});

describe('Asterweft MCP server', () => {
  test('exposes discovery tools with absolute local paths', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'guide', 'setup'), { recursive: true });
    writeFileSync(
      join(root, 'index.md'),
      '---\ntitle: Ignored library root\n---\nRoot-only marker',
    );
    writeFileSync(
      join(root, 'guide', 'index.md'),
      [
        '---',
        'title: 指南',
        'icon: 📘',
        'order: 3',
        'expanded: true',
        'summary: Setup overview',
        '---',
        '# 安装',
        '',
        '安装正文',
      ].join('\n'),
    );
    writeFileSync(
      join(root, 'guide', 'setup', 'index.md'),
      '---\ntitle: 安装步骤\n---\n安装步骤正文',
    );
    writeFileSync(join(root, 'guide', 'reference.pdf'), 'reference');
    writeFileSync(join(root, 'guide', 'setup', 'report.txt'), 'report');
    writeFileSync(join(root, 'guide', 'setup', 'icon.png'), 'image');
    execFileSync('git', ['init', '--quiet'], { cwd: root });
    execFileSync('git', ['config', 'user.name', 'Asterweft Test'], {
      cwd: root,
    });
    execFileSync('git', ['config', 'user.email', 'test@asterweft.local'], {
      cwd: root,
    });
    execFileSync('git', ['add', '--all'], { cwd: root });
    execFileSync('git', ['commit', '--quiet', '-m', 'docs: initialize notes'], {
      cwd: root,
    });
    const library = await DocumentLibrary.open(root, {
      cachePath: join(temporaryLibrary(), 'cache.sqlite'),
      watch: false,
    });
    const resolvedRoot = realpathSync(root);
    const handler = createMcpHandler(() => createAsterweftMcpServer(library), {
      legacy: 'stateless',
      responseMode: 'auto',
    });
    const client = new Client(
      { name: 'asterweft-test', version: '1.0.0' },
      { versionNegotiation: { mode: { pin: '2026-07-28' } } },
    );
    const transport = new StreamableHTTPClientTransport(
      new URL('http://127.0.0.1:3001/mcp'),
      {
        fetch: (input, init) => handler.fetch(new Request(input, init)),
      },
    );

    await client.connect(transport);
    expect(client.getProtocolEra()).toBe('modern');
    const instructions = client.getInstructions() ?? '';
    expect(instructions).toContain('list_articles');
    expect(instructions).toContain('search_articles');
    expect(instructions).toContain('resolve_article');
    expect(instructions).toContain('index.md');
    expect(instructions).toContain('directory');
    expect(instructions).toContain('file tools');
    expect(instructions).not.toContain('read_article');
    expect(instructions).not.toContain('write_article');

    const tools = await client.listTools();
    expect(tools.tools.map((tool) => tool.name)).toEqual([
      'list_articles',
      'search_articles',
      'resolve_article',
    ]);
    const serializedTools = JSON.stringify(tools.tools);
    expect(serializedTools).toContain(
      'absolute local file and directory paths',
    );
    expect(serializedTools).toContain('lineCount');
    expect(serializedTools).toContain('attachmentCount');
    expect(serializedTools).toContain('library-relative article path');
    expect(serializedTools).not.toContain('read_article');
    expect(serializedTools).not.toContain('write_article');
    expect(serializedTools).not.toContain('upload_attachment');
    expect(serializedTools).not.toContain('expectedRevision');
    expect(serializedTools).not.toContain('list_tags');
    expect(serializedTools).not.toMatch(/[\u3400-\u9fff]/u);

    const rootList = await client.callTool({
      name: 'list_articles',
      arguments: { parent: '' },
    });
    expect(rootList.isError).not.toBe(true);
    const listedGuide = (
      rootList.structuredContent as {
        articles: Array<Record<string, unknown>>;
      }
    ).articles[0];
    expect(rootList.structuredContent).toMatchObject({
      parent: '',
      articles: [
        expect.objectContaining({
          path: 'guide',
          title: '指南',
          icon: '📘',
          summary: 'Setup overview',
          lineCount: 3,
          attachmentCount: 1,
          childCount: 1,
          file: join(resolvedRoot, 'guide', 'index.md'),
          directory: join(resolvedRoot, 'guide'),
        }),
      ],
      nextCursor: null,
    });
    expect(listedGuide).not.toHaveProperty('hasChildren');
    expect(listedGuide).not.toHaveProperty('children');
    expect(listedGuide).not.toHaveProperty('modifiedAt');
    expect(listedGuide).not.toHaveProperty('tags');
    expect(listedGuide).not.toHaveProperty('createAt');
    expect(listedGuide).not.toHaveProperty('updateAt');
    expect(listedGuide).not.toHaveProperty('frontmatter');
    expect(listedGuide).not.toHaveProperty('order');
    expect(listedGuide).not.toHaveProperty('expanded');

    const guideChildren = await client.callTool({
      name: 'list_articles',
      arguments: { parent: 'guide' },
    });
    expect(guideChildren.structuredContent).toMatchObject({
      parent: 'guide',
      articles: [
        expect.objectContaining({
          path: 'guide/setup',
          attachmentCount: 1,
          childCount: 0,
          file: join(resolvedRoot, 'guide', 'setup', 'index.md'),
          directory: join(resolvedRoot, 'guide', 'setup'),
        }),
      ],
    });

    const rejectedDepth = await client.callTool({
      name: 'list_articles',
      arguments: { parent: '', depth: 2 },
    });
    expect(rejectedDepth.isError).toBe(true);

    const rejectedSlashParent = await client.callTool({
      name: 'list_articles',
      arguments: { parent: '///' },
    });
    expect(rejectedSlashParent.isError).toBe(true);

    expect(JSON.stringify(rootList.structuredContent)).not.toContain(
      'Ignored library root',
    );

    const rootSearch = await client.callTool({
      name: 'search_articles',
      arguments: { query: 'Root-only marker' },
    });
    expect(rootSearch.structuredContent).toMatchObject({
      total: 0,
      results: [],
    });

    const bodySearch = await client.callTool({
      name: 'search_articles',
      arguments: { query: '安装正文' },
    });
    expect(bodySearch.isError).not.toBe(true);
    expect(bodySearch.structuredContent).toMatchObject({
      total: 1,
      results: [
        expect.objectContaining({
          path: 'guide',
          title: '指南',
          attachmentCount: 1,
          file: join(resolvedRoot, 'guide', 'index.md'),
          directory: join(resolvedRoot, 'guide'),
          excerpt: expect.stringContaining('安装正文'),
        }),
      ],
    });
    const searchHit = (
      bodySearch.structuredContent as {
        results: Array<Record<string, unknown>>;
      }
    ).results[0];
    expect(searchHit).not.toHaveProperty('order');
    expect(searchHit).not.toHaveProperty('expanded');
    expect(searchHit).not.toHaveProperty('frontmatter');

    const missingParent = await client.callTool({
      name: 'list_articles',
      arguments: { parent: 'missing' },
    });
    expect(missingParent.isError).toBe(true);
    expect(missingParent.structuredContent).toEqual({
      error: {
        code: 'ARTICLE_NOT_FOUND',
        message: 'The requested article does not exist.',
        retryable: false,
      },
    });
    expect(JSON.stringify(missingParent.content)).not.toMatch(
      /[\u3400-\u9fff]/u,
    );

    const resolved = await client.callTool({
      name: 'resolve_article',
      arguments: { path: 'guide/setup' },
    });
    expect(resolved.isError).not.toBe(true);
    expect(resolved.structuredContent).toMatchObject({
      path: 'guide/setup',
      title: '安装步骤',
      lineCount: 1,
      attachmentCount: 1,
      childCount: 0,
      file: join(resolvedRoot, 'guide', 'setup', 'index.md'),
      directory: join(resolvedRoot, 'guide', 'setup'),
    });

    const resolvedFromIndex = await client.callTool({
      name: 'resolve_article',
      arguments: { path: 'guide/index.md' },
    });
    expect(resolvedFromIndex.isError).not.toBe(true);
    expect(resolvedFromIndex.structuredContent).toMatchObject({
      path: 'guide',
      title: '指南',
      attachmentCount: 1,
      childCount: 1,
      file: join(resolvedRoot, 'guide', 'index.md'),
      directory: join(resolvedRoot, 'guide'),
    });

    const resolvedFromReference = await client.callTool({
      name: 'resolve_article',
      arguments: { path: 'asterweft://guide/setup' },
    });
    expect(resolvedFromReference.isError).not.toBe(true);
    expect(resolvedFromReference.structuredContent).toMatchObject({
      path: 'guide/setup',
      title: '安装步骤',
      file: join(resolvedRoot, 'guide', 'setup', 'index.md'),
      directory: join(resolvedRoot, 'guide', 'setup'),
    });

    const listedFromReference = await client.callTool({
      name: 'list_articles',
      arguments: { parent: 'asterweft://guide' },
    });
    expect(listedFromReference.isError).not.toBe(true);
    expect(listedFromReference.structuredContent).toMatchObject({
      parent: 'asterweft://guide',
      articles: [
        expect.objectContaining({
          path: 'guide/setup',
          file: join(resolvedRoot, 'guide', 'setup', 'index.md'),
          directory: join(resolvedRoot, 'guide', 'setup'),
        }),
      ],
    });

    const missingResolved = await client.callTool({
      name: 'resolve_article',
      arguments: { path: 'missing' },
    });
    expect(missingResolved.isError).toBe(true);
    expect(missingResolved.structuredContent).toEqual({
      error: {
        code: 'ARTICLE_NOT_FOUND',
        message: 'The requested article does not exist.',
        retryable: false,
      },
    });

    const rejectedRootResolve = await client.callTool({
      name: 'resolve_article',
      arguments: { path: '' },
    });
    expect(rejectedRootResolve.isError).toBe(true);

    await client.close();
    await handler.close();
    library.close();
  });

  test('builds reported paths from the configured rootPath', async () => {
    const root = temporaryLibrary();
    mkdirSync(join(root, 'guide'), { recursive: true });
    writeFileSync(
      join(root, 'guide', 'index.md'),
      '---\ntitle: 指南\n---\n正文',
    );
    execFileSync('git', ['init', '--quiet'], { cwd: root });
    execFileSync('git', ['config', 'user.name', 'Asterweft Test'], {
      cwd: root,
    });
    execFileSync('git', ['config', 'user.email', 'test@asterweft.local'], {
      cwd: root,
    });
    execFileSync('git', ['add', '--all'], { cwd: root });
    execFileSync('git', ['commit', '--quiet', '-m', 'docs: initialize notes'], {
      cwd: root,
    });
    const library = await DocumentLibrary.open(root, {
      cachePath: join(temporaryLibrary(), 'cache.sqlite'),
      watch: false,
    });
    const handler = createMcpHandler(
      () => createAsterweftMcpServer(library, { rootPath: '/library' }),
      { legacy: 'stateless', responseMode: 'auto' },
    );
    const client = new Client(
      { name: 'asterweft-test', version: '1.0.0' },
      { versionNegotiation: { mode: { pin: '2026-07-28' } } },
    );
    const transport = new StreamableHTTPClientTransport(
      new URL('http://127.0.0.1:3001/mcp'),
      {
        fetch: (input, init) => handler.fetch(new Request(input, init)),
      },
    );

    await client.connect(transport);
    const resolved = await client.callTool({
      name: 'resolve_article',
      arguments: { path: 'guide' },
    });
    expect(resolved.structuredContent).toMatchObject({
      path: 'guide',
      file: join('/library', 'guide', 'index.md'),
      directory: join('/library', 'guide'),
    });

    const listed = await client.callTool({
      name: 'list_articles',
      arguments: { parent: '' },
    });
    expect(listed.structuredContent).toMatchObject({
      articles: [
        expect.objectContaining({
          file: join('/library', 'guide', 'index.md'),
          directory: join('/library', 'guide'),
        }),
      ],
    });

    await client.close();
    await handler.close();
    library.close();
  });
});
