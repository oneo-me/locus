import { createElement, type ComponentProps, type ReactNode } from 'react';
import { renderToReadableStream } from 'react-dom/server';
import { MemoryRouter } from 'react-router';
import { describe, expect, test } from 'vitest';
import { MarkdownContent as RenderedMarkdownContent } from './markdown';
import { parseDocumentMarkdown } from './markdown-parser';

async function MarkdownContent({
  assetImageSizes = {},
  content,
  ...props
}: Omit<ComponentProps<typeof RenderedMarkdownContent>, 'content'> & {
  assetImageSizes?: Parameters<
    typeof parseDocumentMarkdown
  >[0]['assetImageSizes'];
  content: string;
}) {
  const tree = await parseDocumentMarkdown({
    assetImageSizes,
    assets: props.assets,
    content,
    documentPath: props.documentPath,
  });
  return <RenderedMarkdownContent {...props} content={tree} />;
}

async function renderMarkup(node: ReactNode): Promise<string> {
  const stream = await renderToReadableStream(node);
  await stream.allReady;
  return new Response(stream).text();
}

describe('MarkdownContent', () => {
  test('lets the reading column follow the document surface width', async () => {
    const markup = await renderMarkup(
      <MarkdownContent
        assets={[]}
        content="正文"
        documentPath="guide/index.md"
      />,
    );

    expect(markup).toContain('w-full');
    expect(markup).toContain('space-y-5');
    expect(markup).not.toContain('max-w-[');
  });

  test('renders sibling images and links through the attachment route', async () => {
    const markup = await renderMarkup(
      <MarkdownContent
        assets={['076-001-卡号列表.png', '说明.pdf']}
        content={[
          '![卡号列表](076-001-卡号列表.png)',
          '',
          '[下载说明](./说明.pdf)',
        ].join('\n')}
        documentPath="archives/personal-archives/bank-cards/index.md"
      />,
    );

    expect(markup).toContain(
      'src="/api/assets?path=archives%2Fpersonal-archives%2Fbank-cards%2F076-001-%E5%8D%A1%E5%8F%B7%E5%88%97%E8%A1%A8.png&amp;thumbnail=1"',
    );
    expect(markup).toContain(
      'data-document-export-image-source="/api/assets?path=archives%2Fpersonal-archives%2Fbank-cards%2F076-001-%E5%8D%A1%E5%8F%B7%E5%88%97%E8%A1%A8.png"',
    );
    expect(markup).toContain('aria-label="查看原图：卡号列表"');
    expect(markup).toContain(
      'href="/api/assets?path=archives%2Fpersonal-archives%2Fbank-cards%2F%E8%AF%B4%E6%98%8E.pdf"',
    );
    expect(markup).not.toContain(
      'src="076-001-%E5%8D%A1%E5%8F%B7%E5%88%97%E8%A1%A8.png"',
    );
    expect(markup).not.toContain(
      'src="/api/assets?path=archives%2Fpersonal-archives%2Fbank-cards%2F076-001-%E5%8D%A1%E5%8F%B7%E5%88%97%E8%A1%A8.png"',
    );
  });

  test('does not rewrite external images', async () => {
    const markup = await renderMarkup(
      <MarkdownContent
        assets={['diagram.png']}
        content="![外部图片](https://example.com/diagram.png)"
        documentPath="guide/index.md"
      />,
    );

    expect(markup).toContain('src="https://example.com/diagram.png"');
  });

  test('renders a large local image gallery without visible chrome', async () => {
    const assets = Array.from(
      { length: 16 },
      (_, index) => `sample-${String(index + 1).padStart(2, '0')}.jpg`,
    );
    const content = [
      '::gallery',
      ...assets.map((asset, index) => `![样本 ${index + 1}](${asset})`),
      '::',
    ].join('\n');
    const markup = await renderMarkup(
      <MarkdownContent
        assetImageSizes={Object.fromEntries(
          assets.map((asset) => [asset, { height: 300, width: 400 }]),
        )}
        assets={assets}
        content={content}
        documentPath="guide/index.md"
      />,
    );

    expect(markup).toContain('data-attachment-gallery="true"');
    expect(markup).toContain('class="document-adaptive-block"');
    expect(markup).toContain('aria-label="图片附件，共 16 张"');
    expect(markup).toContain('flex-wrap');
    expect(markup).toContain('data-gallery-height="300"');
    expect(markup).toContain('data-gallery-width="400"');
    expect(markup).not.toContain('columns-2');
    expect(markup).not.toContain('sm:columns-3');
    expect(markup).not.toContain('row-span-2');
    expect(markup.match(/loading="lazy"/g)).toHaveLength(16);
    expect(markup.match(/thumbnail=1/g)).toHaveLength(16);
    expect(markup.match(/aria-label="查看图片：样本 /g)).toHaveLength(16);
    expect(markup).not.toContain('无法显示图库');
    expect(markup).not.toContain('>图片附件<');
  });

  test('renders only explicitly referenced ordinary attachments', async () => {
    const markup = await renderMarkup(
      <MarkdownContent
        assets={['brief.pdf', 'data.csv', 'hidden.zip']}
        content={[
          '::attachments',
          '[brief.pdf](brief.pdf)',
          '[data.csv](data.csv)',
          '::',
        ].join('\n')}
        documentPath="guide/index.md"
      />,
    );

    expect(markup).toContain('data-attachment-list="true"');
    expect(markup).toContain('在新标签页打开附件：brief.pdf');
    expect(markup).toContain('下载附件：data.csv');
    expect(markup).not.toContain('hidden.zip');
    expect(markup).not.toContain('无法显示附件');
    expect(markup).not.toContain('data-document-export-exclude');
  });

  test('shows actionable errors for invalid attachment containers', async () => {
    const markup = await renderMarkup(
      <MarkdownContent
        assets={['photo.png', 'brief.pdf']}
        content={[
          '::gallery',
          '![缺少文件](missing.png)',
          '::',
          '',
          '::attachments',
          '[自定义名称](brief.pdf)',
          '::',
          '',
          '::attachments',
          '[photo.png](photo.png)',
          '::',
        ].join('\n')}
        documentPath="guide/index.md"
      />,
    );

    expect(markup.match(/role="alert"/g)).toHaveLength(3);
    expect(markup).toContain('无法显示图库');
    expect(markup.match(/无法显示附件/g)).toHaveLength(2);
    expect(markup).not.toContain('data-attachment-gallery');
    expect(markup).not.toContain('data-attachment-list');
  });

  test('renders Markdown links like internal links and opens remote links with a favicon', async () => {
    const markup = await renderMarkup(
      <MarkdownContent
        assets={[]}
        content={[
          '[远端文档](https://example.com/guide?lang=zh)',
          '',
          '[页内章节](#details)',
        ].join('\n')}
        documentPath="guide/index.md"
      />,
    );

    expect(markup.match(/class="document-inline-link"/g)).toHaveLength(2);
    expect(markup).not.toContain('data-slot="badge"');
    expect(markup).toContain(
      'href="https://example.com/guide?lang=zh" rel="noopener noreferrer" target="_blank"',
    );
    expect(markup).toContain(
      'src="https://www.google.com/s2/favicons?domain_url=https%3A%2F%2Fexample.com&amp;sz=32"',
    );
    expect(markup).toContain('href="#details"');
    expect(markup).not.toContain('href="#details" rel=');
  });

  test('only parses explicit Markdown and wiki links', async () => {
    const markup = await renderMarkup(
      <MemoryRouter>
        <MarkdownContent
          assets={[]}
          content={[
            '## AGENTS.md',
            '',
            '裸地址 https://example.com/guide 保持为文本。',
            '',
            '[Markdown 链接](https://example.com/guide)',
            '',
            '内部链接 [[operations/video-workflow]]',
          ].join('\n')}
          documentPath="guide/index.md"
          documents={[
            {
              path: 'operations/video-workflow/index.md',
              title: '视频工作流',
            },
          ]}
        />
      </MemoryRouter>,
    );

    expect(markup).toMatch(/<h2[^>]*>AGENTS\.md<\/h2>/);
    expect(markup).toContain('裸地址 https://example.com/guide 保持为文本。');
    expect(markup.match(/href="https:\/\/example.com\/guide"/g)).toHaveLength(
      1,
    );
    expect(markup).toContain('href="/operations/video-workflow"');
  });

  test('renders code blocks without syntax highlighting and keeps language labels', async () => {
    const markup = await renderMarkup(
      <MarkdownContent
        assets={[]}
        content={[
          '```typescript',
          'const woven = true;',
          '```',
          '',
          '```',
          'plain text',
          '```',
        ].join('\n')}
        documentPath="guide/index.md"
      />,
    );

    expect(markup).toContain(
      'class="code-block document-adaptive-block rendered-block"',
    );
    expect(markup).toContain(
      '<figcaption class="rendered-block__label">typescript</figcaption>',
    );
    expect(markup).toContain(
      '<figcaption class="rendered-block__label">text</figcaption>',
    );
    expect(markup).toContain('const woven = true;');
    expect(markup).not.toContain('style="color:');
    expect(markup).not.toContain('class="line"');
  });

  test('renders a canvas inline across the note pane', async () => {
    const markup = await renderMarkup(
      <MarkdownContent
        assets={[]}
        content={[
          '::canvas',
          '```html',
          '<button class="rounded bg-sky-700 px-4 py-2 text-white" onclick="this.dataset.clicked = \'true\'">保存</button>',
          "<script>document.body.dataset.canvasScript = 'ready'</script>",
          '```',
          '::',
        ].join('\n')}
        documentPath="guide/index.md"
      />,
    );

    expect(markup).toContain('data-canvas-container="true"');
    expect(markup).toContain('canvas-block document-wide-block');
    expect(markup).not.toContain('overflow-hidden');
    expect(markup).not.toContain('rounded-[');
    expect(markup).not.toContain(' bg-card');
    expect(markup).toContain('data-canvas-frame="true"');
    expect(markup).toContain('title="画布"');
    expect(markup).not.toContain('<figcaption');
    expect(markup).not.toContain('在新页面打开');
    expect(markup).not.toContain('&lt;button');
    expect(markup).not.toContain('canvasScript');
  });

  test('ignores the obsolete canvas height attribute', async () => {
    const markup = await renderMarkup(
      <MarkdownContent
        assets={[]}
        content={[
          '::canvas{height="1000"}',
          '```html',
          '<p>高度由内部内容决定</p>',
          '```',
          '::',
        ].join('\n')}
        documentPath="guide/index.md"
      />,
    );

    expect(markup.match(/data-canvas-container="true"/g)).toHaveLength(1);
    expect(markup).not.toContain('data-canvas-height');
    expect(markup).not.toContain('role="alert"');
    expect(markup.match(/<iframe/g)).toHaveLength(1);
  });

  test('shows an actionable error for an invalid canvas region', async () => {
    const markup = await renderMarkup(
      <MarkdownContent
        assets={[]}
        content={[
          '::canvas',
          '<style>body { color: red }</style>',
          '<h1>未使用代码围栏</h1>',
          '::',
        ].join('\n')}
        documentPath="guide/index.md"
      />,
    );

    expect(markup).toContain('role="alert"');
    expect(markup).toContain('无法显示画布');
    expect(markup).toContain(
      '请在 ::canvas 区域中仅放置一个 html fenced code block。',
    );
    expect(markup).not.toContain('<iframe');
  });

  test('does not support the former tailwind container syntax', async () => {
    const markup = await renderMarkup(
      <MarkdownContent
        assets={[]}
        content={[
          '::tailwind{title="旧语法" description="不再支持"}',
          '```html',
          '<p>旧画布</p>',
          '```',
          '::',
        ].join('\n')}
        documentPath="guide/index.md"
      />,
    );

    expect(markup).toContain('data-unsupported-markdown-component="tailwind"');
    expect(markup).not.toContain('data-canvas-trigger');
    expect(markup).not.toContain('<iframe');
  });

  test('preserves GFM table column alignment', async () => {
    const markup = await renderMarkup(
      <MarkdownContent
        assets={[]}
        content={[
          '| 左对齐 | 居中 | 右对齐 |',
          '| :--- | :---: | ---: |',
          '| 内容 | 内容 | 内容 |',
        ].join('\n')}
        documentPath="guide/index.md"
      />,
    );

    expect(markup).toContain('<table>');
    expect(markup).toContain('<th style="text-align:left">左对齐</th>');
    expect(markup).toContain('<th style="text-align:center">居中</th>');
    expect(markup).toContain('<th style="text-align:right">右对齐</th>');
    expect(markup).toContain('<td style="text-align:left">内容</td>');
    expect(markup).toContain('<td style="text-align:center">内容</td>');
    expect(markup).toContain('<td style="text-align:right">内容</td>');
  });

  test('preserves the note typeset hooks for GFM task lists', async () => {
    const markup = await renderMarkup(
      <MarkdownContent
        assets={[]}
        content={['- [x] 已完成', '- [ ] 待处理'].join('\n')}
        documentPath="guide/index.md"
      />,
    );

    expect(markup).toContain('<ul class="contains-task-list">');
    expect(markup).toContain('<li class="task-list-item">');
    expect(markup).toContain('type="checkbox"');
  });

  test('keeps CJK emphasis and GFM semantics after the migration', async () => {
    const markup = await renderMarkup(
      <MarkdownContent
        assets={[]}
        content={['**中文重点**与普通文本。', '', '$$', 'E = mc^2', '$$'].join(
          '\n',
        )}
        documentPath="guide/index.md"
      />,
    );

    expect(markup).toContain('<strong>中文重点</strong>');
    expect(markup).not.toContain('class="math block"');
    expect(markup).not.toContain('<math');
  });

  test('does not render raw HTML or unregistered Comark tags as executable elements', async () => {
    const markup = await renderMarkup(
      <MarkdownContent
        assets={[]}
        content={[
          '<script>globalThis.compromised = true</script>',
          '',
          '::script',
          '仍然保留可读内容。',
          '::',
          '',
          '::html',
          '```html',
          '<button>旧预览</button>',
          '```',
          '::',
        ].join('\n')}
        documentPath="guide/index.md"
      />,
    );

    expect(markup).not.toContain('<script');
    expect(markup).not.toContain('data-canvas');
    expect(markup).toContain('仍然保留可读内容');
    expect(markup).toContain('data-unsupported-markdown-component="script"');
    expect(markup).toContain('data-unsupported-markdown-component="html"');
  });
});

describe('MarkdownContent wiki links', () => {
  const documents = [
    { icon: '💬', path: 'operations/index.md', title: '运营' },
    {
      icon: '🎞️',
      path: 'operations/video-workflow/index.md',
      title: '视频工作流',
    },
  ];

  test('renders resolved wiki links with the shared inline style, title, and icon', async () => {
    const markup = await renderMarkup(
      createElement(
        MemoryRouter,
        null,
        createElement(MarkdownContent, {
          assets: [],
          content: '见 [[operations/video-workflow]]。',
          documentPath: 'guide/index.md',
          documents,
        }),
      ),
    );

    expect(markup).toContain('href="/operations/video-workflow"');
    expect(markup).toContain('title="运营 / 视频工作流"');
    expect(markup).toContain('class="document-inline-link"');
    expect(markup).not.toContain('data-slot="badge"');
    expect(markup).toContain('🎞️');
    expect(markup).toContain('>视频工作流</a>');
    expect(markup).not.toContain('operations/video-workflow</a>');
  });

  test('marks missing wiki links with the shared invalid style', async () => {
    const markup = await renderMarkup(
      createElement(
        MemoryRouter,
        null,
        createElement(MarkdownContent, {
          assets: [],
          content: [
            '缺失 [[operations/missing]]',
            '',
            '```',
            '[[operations/video-workflow]]',
            '```',
          ].join('\n'),
          documentPath: 'guide/index.md',
          documents,
        }),
      ),
    );

    expect(markup).toContain(
      'class="document-inline-link" data-invalid="true"',
    );
    expect(markup).not.toContain('data-slot="badge"');
    expect(markup).toContain('title="未找到笔记：operations/missing"');
    expect(markup).toContain('>operations/missing<');
    expect(markup).toContain('[[operations/video-workflow]]');
    expect(markup).not.toContain('href="/operations/video-workflow"');
  });
});
