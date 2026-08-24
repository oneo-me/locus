import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, test } from 'vitest';
import {
  AttachmentGallery,
  AttachmentList,
  splitFilenameForMiddleEllipsis,
} from './attachment-views';

describe('AttachmentGallery', () => {
  test('renders final aspect ratios in the initial markup', () => {
    const markup = renderToStaticMarkup(
      <AttachmentGallery
        documentPath="guide/index.md"
        items={Array.from({ length: 7 }, (_, index) => ({
          alt: `样本 ${index + 1}`,
          height: index === 0 ? 640 : 300,
          name: `sample-${index + 1}.jpg`,
          width: index === 0 ? 213 : 400,
        }))}
      />,
    );

    expect(markup).toContain('flex-wrap');
    expect(markup).toContain('class="document-adaptive-block"');
    expect(markup).toContain('after:grow-[999999]');
    expect(markup).toContain('data-gallery-height="640"');
    expect(markup).toContain('data-gallery-width="213"');
    expect(markup).toContain('aspect-ratio:213 / 640');
    expect(markup).toContain('height="640"');
    expect(markup).toContain('width="213"');
    expect(markup).not.toContain('columns-2');
    expect(markup).not.toContain('sm:columns-3');
    expect(markup).not.toContain('break-inside-avoid');
    expect(markup.match(/thumbnail=1/g)).toHaveLength(7);
    expect(markup.match(/data-document-export-image-source=/g)).toHaveLength(7);
    expect(markup).toContain(
      'data-document-export-image-source="/api/assets?path=guide%2Fsample-1.jpg"',
    );
    expect(markup).not.toContain('src="/api/assets?path=guide%2Fsample-1.jpg"');
    expect(markup).not.toContain('row-span-2');
    expect(markup).not.toContain('col-span-');
  });

  test('rejects gallery items without server-provided dimensions', () => {
    const markup = renderToStaticMarkup(
      <AttachmentGallery
        documentPath="guide/index.md"
        items={[{ alt: '没有尺寸', name: 'sample.jpg' }]}
      />,
    );

    expect(markup).toContain('无法显示图库');
    expect(markup).not.toContain('data-attachment-gallery');
  });
});

describe('AttachmentList', () => {
  test('opens browser-viewable attachments and downloads other files', () => {
    const markup = renderToStaticMarkup(
      <AttachmentList
        documentPath="guide/index.md"
        items={[
          { name: '说明.pdf' },
          { name: 'report.html' },
          { name: 'archive.zip' },
        ]}
      />,
    );

    expect(markup).toContain('aria-label="普通附件"');
    expect(markup).toContain('在新标签页打开附件：说明.pdf');
    expect(markup).toContain('在新标签页打开附件：report.html');
    expect(markup).toContain('下载附件：archive.zip');
    expect(markup).toContain('title="说明.pdf"');
    expect(markup).toContain('target="_blank"');
    expect(markup).toContain('rel="noopener noreferrer"');
    expect(markup).toContain('download="archive.zip"');
    expect(markup).toContain('grid-cols-1');
    expect(markup).not.toContain('sm:grid-cols-2');
    expect(markup).toContain('text-foreground');
    expect(markup).not.toContain('>打开<');
    expect(markup).not.toContain('>下载<');
    expect(markup).not.toContain('data-document-export-exclude');
    expect(markup).not.toContain('>普通附件<');
  });

  test('keeps both ends of long file names available for middle truncation', () => {
    expect(
      splitFilenameForMiddleEllipsis(
        'product-research-notes-for-second-quarter-final.pdf',
      ),
    ).toEqual(['product-research-notes-for-second-quart', 'er-final.pdf']);
    expect(splitFilenameForMiddleEllipsis('说明.pdf')).toEqual([
      '说明.pdf',
      '',
    ]);
  });
});
