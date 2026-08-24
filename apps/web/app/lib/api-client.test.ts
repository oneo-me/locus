import { afterEach, describe, expect, test, vi } from 'vitest';
import { apiGet } from './api-client';

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('API client caching', () => {
  test('bypasses the browser cache for dynamic reads', async () => {
    const fetchMock = vi.fn().mockResolvedValue(Response.json({ ok: true }));
    vi.stubGlobal('fetch', fetchMock);

    await expect(apiGet('/api/library', { q: '更新' })).resolves.toEqual({
      ok: true,
    });
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/library?q=%E6%9B%B4%E6%96%B0',
      { cache: 'no-store' },
    );
  });
});
