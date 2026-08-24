import { describe, expect, test } from 'vitest';
import { createWebAppManifestResponse, webAppManifest } from './manifest';

describe('web app manifest', () => {
  test('returns application manifest JSON', async () => {
    const response = createWebAppManifestResponse(
      new Request('http://asterweft.test/manifest'),
    );

    expect(response.status).toBe(200);
    expect(response.headers.get('content-type')).toContain(
      'application/manifest+json',
    );
    expect(await response.json()).toEqual(webAppManifest);
  });
});
