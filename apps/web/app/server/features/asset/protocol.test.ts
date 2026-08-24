import { describe, expect, test } from 'vitest';
import {
  getAssetContentDisposition,
  getAssetContentType,
  getAssetThumbnailUrl,
  getAssetUrl,
  getMarkdownAssetName,
  isBrowserViewableAsset,
  isPreviewableImageAsset,
  parseAssetPath,
  resolveMarkdownAssetUrl,
} from './protocol';

describe('library assets', () => {
  test('builds attachment API URLs', () => {
    expect(getAssetUrl('guide/index.md', '示意 图.png')).toBe(
      '/api/assets?path=guide%2F%E7%A4%BA%E6%84%8F+%E5%9B%BE.png',
    );
    expect(getAssetUrl('index.md', '示意 图.png')).toBe(
      '/api/assets?path=%E7%A4%BA%E6%84%8F+%E5%9B%BE.png',
    );
    expect(getAssetThumbnailUrl('guide/index.md', '示意 图.png')).toBe(
      '/api/assets?path=guide%2F%E7%A4%BA%E6%84%8F+%E5%9B%BE.png&thumbnail=1',
    );
  });

  test('resolves one attachment path to its document and filename', () => {
    expect(parseAssetPath('guide/images/示意图.png')).toEqual({
      documentPath: 'guide/images/index.md',
      assetName: '示意图.png',
    });
    expect(parseAssetPath('说明.pdf')).toEqual({
      documentPath: 'index.md',
      assetName: '说明.pdf',
    });
    expect(parseAssetPath('../说明.pdf')).toBeNull();
    expect(parseAssetPath('guide//说明.pdf')).toBeNull();
  });

  test('identifies previewable image types without sniffing unknown files', () => {
    expect(isPreviewableImageAsset('PHOTO.JPEG')).toBe(true);
    expect(isPreviewableImageAsset('report.pdf')).toBe(false);
    expect(isBrowserViewableAsset('PHOTO.JPEG')).toBe(true);
    expect(isBrowserViewableAsset('report.pdf')).toBe(true);
    expect(isBrowserViewableAsset('report.html')).toBe(true);
    expect(isBrowserViewableAsset('archive.zip')).toBe(false);
    expect(getAssetContentType('unknown.bin')).toBe('application/octet-stream');
  });

  test('encodes filenames safely for content disposition', () => {
    expect(getAssetContentDisposition('设计稿 (最终).png', false)).toBe(
      'attachment; filename="___ (__).png"; filename*=UTF-8\'\'%E8%AE%BE%E8%AE%A1%E7%A8%BF%20%28%E6%9C%80%E7%BB%88%29.png',
    );
  });

  test('resolves sibling Markdown assets against the owning document', () => {
    const documentPath = 'archives/personal-archives/bank-cards/index.md';
    const assets = ['076-001-卡号列表.png', '说明 文档.pdf'];

    expect(
      resolveMarkdownAssetUrl(documentPath, assets, '076-001-卡号列表.png'),
    ).toBe(
      '/api/assets?path=archives%2Fpersonal-archives%2Fbank-cards%2F076-001-%E5%8D%A1%E5%8F%B7%E5%88%97%E8%A1%A8.png',
    );
    expect(
      getMarkdownAssetName(
        documentPath,
        assets,
        './%E8%AF%B4%E6%98%8E%20%E6%96%87%E6%A1%A3.pdf',
      ),
    ).toBe('说明 文档.pdf');
    expect(
      getMarkdownAssetName(
        documentPath,
        assets,
        '/archives/personal-archives/bank-cards/076-001-%E5%8D%A1%E5%8F%B7%E5%88%97%E8%A1%A8.png',
      ),
    ).toBe('076-001-卡号列表.png');
  });

  test('leaves external and non-attachment Markdown URLs unchanged', () => {
    const documentPath = 'guide/index.md';
    const assets = ['diagram.png'];

    for (const source of [
      'https://example.com/diagram.png',
      '//example.com/diagram.png',
      'data:image/png;base64,AAAA',
      '#diagram',
      '../diagram.png',
      'missing.png',
    ]) {
      expect(resolveMarkdownAssetUrl(documentPath, assets, source)).toBe(
        source,
      );
    }
  });
});
