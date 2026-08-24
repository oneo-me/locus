import { describe, expect, test } from 'vitest';
import {
  documentPathToUrlPath,
  pathnameToUrlPath,
  urlPathToDocumentPath,
  urlPathToHref,
} from './route';

describe('documentPathToUrlPath', () => {
  test('maps index documents to their directory URL', () => {
    expect(documentPathToUrlPath('index.md')).toBe('');
    expect(documentPathToUrlPath('guide/index.md')).toBe('guide');
    expect(documentPathToUrlPath('guide/setup/index.md')).toBe('guide/setup');
  });
});

describe('urlPathToDocumentPath', () => {
  test('maps directory URLs back to index documents', () => {
    expect(urlPathToDocumentPath()).toBe('index.md');
    expect(urlPathToDocumentPath('guide')).toBe('guide/index.md');
    expect(urlPathToDocumentPath('/guide/setup/')).toBe('guide/setup/index.md');
  });
});

describe('direct route paths', () => {
  test('decodes browser pathnames and encodes direct hrefs segment by segment', () => {
    expect(pathnameToUrlPath('/项目/设计%20系统/')).toBe('项目/设计 系统');
    expect(urlPathToHref('项目/设计 系统')).toBe(
      '/%E9%A1%B9%E7%9B%AE/%E8%AE%BE%E8%AE%A1%20%E7%B3%BB%E7%BB%9F',
    );
  });

  test('keeps malformed escape sequences for the library validator to reject', () => {
    expect(pathnameToUrlPath('/guide/%E0%A4%A')).toBe('guide/%E0%A4%A');
  });
});
