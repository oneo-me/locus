export const dynamicContentCacheControl = 'private, no-store';

export function dynamicContentHeaders(init?: HeadersInit): Headers {
  const headers = new Headers(init);
  headers.set('Cache-Control', dynamicContentCacheControl);
  return headers;
}
