export const webAppManifest = {
  id: '/',
  name: 'Asterweft',
  short_name: 'Asterweft',
  description: 'Knowledge, woven together.',
  lang: 'zh-CN',
  start_url: '/',
  scope: '/',
  display: 'standalone',
  background_color: '#f7f9fc',
  theme_color: '#f7f9fc',
  icons: [
    {
      src: '/icons/asterweft-192.png',
      sizes: '192x192',
      type: 'image/png',
      purpose: 'any',
    },
    {
      src: '/icons/asterweft-512.png',
      sizes: '512x512',
      type: 'image/png',
      purpose: 'any',
    },
  ],
} as const;

export function createWebAppManifestResponse(request: Request): Response {
  return new Response(
    request.method === 'HEAD' ? null : JSON.stringify(webAppManifest),
    {
      headers: {
        // Installed PWAs periodically re-fetch this URL to update launch
        // metadata. Always revalidate so route changes are not hidden behind
        // a stale manifest.
        'cache-control': 'no-cache, max-age=0, must-revalidate',
        'content-type': 'application/manifest+json; charset=utf-8',
      },
    },
  );
}
