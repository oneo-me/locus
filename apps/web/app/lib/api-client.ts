export async function readApiResponse<T>(
  response: Promise<Response>,
): Promise<T> {
  const resolved = await response;
  const contentType = resolved.headers.get('content-type') ?? '';
  const isJson = contentType.includes('application/json');
  const payload = isJson ? await resolved.json() : await resolved.text();

  if (!resolved.ok) {
    if (
      payload &&
      typeof payload === 'object' &&
      'error' in payload &&
      typeof (payload as { error: unknown }).error === 'string'
    ) {
      throw new Error((payload as { error: string }).error);
    }
    throw new Error(
      typeof payload === 'string' && payload
        ? payload
        : `请求失败（${resolved.status}）。`,
    );
  }

  return payload as T;
}

export function apiUrl(
  path: string,
  query?: Record<string, string | undefined>,
) {
  const url = new URL(path, 'http://asterweft.local');
  if (query) {
    for (const [key, value] of Object.entries(query)) {
      if (value !== undefined) url.searchParams.set(key, value);
    }
  }
  return `${url.pathname}${url.search}`;
}

export async function apiGet<T>(
  path: string,
  query?: Record<string, string | undefined>,
): Promise<T> {
  return readApiResponse<T>(
    fetch(apiUrl(path, query), {
      cache: 'no-store',
    }),
  );
}

export async function apiSend<T>(
  path: string,
  method: string,
  body?: unknown,
): Promise<T> {
  return readApiResponse<T>(
    fetch(path, {
      method,
      headers:
        body === undefined ? undefined : { 'content-type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
    }),
  );
}
