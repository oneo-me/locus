import type { DocumentContent } from '~/contracts/library-types';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, test } from 'vitest';
import {
  copyDocumentText,
  DocumentActions,
  DocumentBrowser,
  DocumentYamlMetadata,
  formatDocumentReference,
} from './browser';

const document: DocumentContent = {
  path: 'guide/index.md',
  directory: 'guide',
  title: 'guide',
  icon: null,
  iconImageModifiedAt: null,
  metadataKeys: ['title'],
  summary: null,
  assets: [],
  modifiedAt: '2026-07-23T00:00:00.000Z',
  size: 1,
  wordCount: 1,
  issue: null,
  content: '',
};

const participatingInGraph = {
  configured: true,
  inheritedFrom: null,
  participates: true,
};

function renderMetadata(overrides: Partial<DocumentContent> = {}) {
  return renderToStaticMarkup(
    <DocumentYamlMetadata
      document={{ ...document, ...overrides }}
      error={null}
      onImageRemove={() => {}}
      onImageUpload={() => {}}
      onSave={() => {}}
      pending={false}
    />,
  );
}

describe('DocumentBrowser', () => {
  test('keeps the root route surface blank when no article is selected', () => {
    const data = {
      documents: [document],
      search: null,
      selected: null,
      selectedPath: null,
      selectionError: null,
      status: { rootPath: '/notes' },
    } as unknown as Parameters<typeof DocumentBrowser>[0]['data'];

    const markup = renderToStaticMarkup(<DocumentBrowser data={data} />);

    expect(markup).toContain('id="main-content"');
    expect(markup).not.toContain('尚未选择文章');
    expect(markup).not.toContain('无法打开文档');
    expect(markup).not.toContain('文档库还是空的');
  });
});

describe('DocumentActions', () => {
  test('renders responsive image, reference, and settings actions', () => {
    const markup = renderToStaticMarkup(
      <DocumentActions
        documentPath="guide/index.md"
        graphParticipation={participatingInGraph}
        path="guide"
        title="入门指南"
      />,
    );

    const imageIndex = markup.indexOf('aria-label="复制为图片"');
    const downloadIndex = markup.indexOf('aria-label="下载图片"');
    const referenceIndex = markup.indexOf('aria-label="复制文章引用"');
    const settingsIndex = markup.indexOf('aria-label="笔记设置"');
    expect(imageIndex).toBeGreaterThan(-1);
    expect(downloadIndex).toBeGreaterThan(imageIndex);
    expect(referenceIndex).toBeGreaterThan(downloadIndex);
    expect(settingsIndex).toBeGreaterThan(referenceIndex);
    expect(markup).toContain('title="笔记设置"');
    expect(markup).toContain('data-document-export-exclude="true"');
    expect(markup).toContain('items-center justify-end gap-1.5');
    expect(markup).not.toContain('aria-label="是否展示文章差异"');
    expect(markup.slice(imageIndex, downloadIndex)).toContain('phone-hidden');
    expect(markup.slice(downloadIndex, referenceIndex)).toContain('phone-only');
    expect(markup.slice(referenceIndex, settingsIndex)).toContain(
      'phone-hidden',
    );
    expect(markup).not.toContain('absolute top-3 right-3');
  });

  test('copies text and handles unavailable or rejected clipboard access', async () => {
    let copied = '';
    const clipboard = {
      writeText: async (value: string) => {
        copied = value;
      },
    };

    await expect(copyDocumentText('guide', clipboard)).resolves.toBe(true);
    expect(copied).toBe('guide');
    await expect(copyDocumentText('guide', undefined)).resolves.toBe(false);
    await expect(
      copyDocumentText('guide', {
        writeText: async () => {
          throw new Error('Clipboard permission denied');
        },
      }),
    ).resolves.toBe(false);
  });

  test('formats an Asterweft article URI', () => {
    expect(formatDocumentReference('projects/asterweft')).toBe(
      'asterweft://projects/asterweft',
    );
  });
});

describe('DocumentYamlMetadata', () => {
  test('shows summary text without yaml key labels', () => {
    const markup = renderMetadata({
      icon: '📘',
      metadataKeys: ['title', 'icon', 'summary'],
      summary: '短摘要',
      title: '入门指南',
    });

    expect(markup).toContain('<h1');
    expect(markup).toContain('入门指南');
    expect(markup).toContain('📘');
    expect(markup).not.toContain('aria-label="文档设置"');
    expect(markup).toContain('aria-label="摘要"');
    expect(markup).toContain('w-full');
    expect(markup).toContain('backdrop-blur-[3px]');
    expect(markup).toContain('doc-yaml-panel');
    expect(markup).toContain('--doc-yaml-hue:');
    expect(markup).toContain('短摘要');
    expect(markup).not.toContain('>summary<');
    expect(markup).not.toContain('>title<');
    expect(markup).not.toContain('>icon<');
    expect(markup).not.toContain('aria-label="YAML 属性"');
  });

  test('always renders the editable icon beside the title', () => {
    const markup = renderMetadata({ metadataKeys: [] });

    expect(markup).toContain('aria-label="文档图标"');
    expect(markup).toContain('size-8');
    expect(markup).toContain('size-6');
    expect(markup).toContain('text-xl');
    expect(markup).toContain('border-0');
    expect(markup).toContain('<h1');
    expect(markup).toContain('text-xl leading-7 font-medium');
    expect(markup).toContain('guide');
    expect(markup).not.toContain('>icon<');
  });

  test('renders document actions at the right edge of the title row', () => {
    const markup = renderToStaticMarkup(
      <DocumentYamlMetadata
        actions={
          <DocumentActions
            documentPath="guide/index.md"
            graphParticipation={participatingInGraph}
            path="guide"
            title="入门指南"
          />
        }
        document={{ ...document, title: '入门指南' }}
        error={null}
        onImageRemove={() => {}}
        onImageUpload={() => {}}
        onSave={() => {}}
        pending={false}
      />,
    );

    const titleIndex = markup.indexOf('data-document-export-title');
    const actionsIndex = markup.indexOf('aria-label="复制为图片"');
    expect(titleIndex).toBeGreaterThan(-1);
    expect(actionsIndex).toBeGreaterThan(titleIndex);
    expect(markup).toContain('grid-cols-[auto_minmax(0,1fr)_auto]');
    expect(markup).toContain('col-start-2 min-w-0');
    expect(markup).toContain('col-start-3 min-w-0');
  });

  test('renders a custom image icon and keeps its upload action in the picker', () => {
    const markup = renderMetadata({
      icon: '📘',
      iconImageModifiedAt: '2026-07-26T00:00:00.000Z',
    });

    expect(markup).toContain('自定义图片图标');
    expect(markup).toContain(
      '/api/assets?path=guide%2Ficon.png&amp;v=2026-07-26T00%3A00%3A00.000Z',
    );
  });

  test('does not render a settings trigger on the title row', () => {
    const markup = renderMetadata({ metadataKeys: [] });

    expect(markup).not.toContain('aria-label="文档设置"');
  });

  test('hides summary area when summary is empty', () => {
    const markup = renderMetadata({
      metadataKeys: ['title', 'icon'],
      summary: null,
    });

    expect(markup).not.toContain('aria-label="摘要"');
    expect(markup).not.toContain('doc-yaml-panel');
  });
});
