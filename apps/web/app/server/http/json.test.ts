import { describe, expect, test } from 'vitest';
import { dynamicContentCacheControl } from './cache';
import { jsonError, jsonOk } from './json';

describe('JSON response caching', () => {
  test('prevents successful dynamic responses from being cached', async () => {
    const response = await jsonOk(
      { ok: true },
      { headers: { 'X-Test': 'preserved' } },
    );

    expect(response.headers.get('cache-control')).toBe(
      dynamicContentCacheControl,
    );
    expect(response.headers.get('x-test')).toBe('preserved');
  });

  test('prevents error responses from being cached', async () => {
    const response = await jsonError(new Error('失败'));

    expect(response.headers.get('cache-control')).toBe(
      dynamicContentCacheControl,
    );
  });
});
