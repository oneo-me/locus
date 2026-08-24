import { dynamicContentHeaders } from './cache';

export async function jsonOk<T>(
  data: T,
  init?: ResponseInit,
): Promise<Response> {
  return Response.json(data, {
    status: 200,
    ...init,
    headers: dynamicContentHeaders(init?.headers),
  });
}

export async function jsonError(
  error: unknown,
  status = 400,
): Promise<Response> {
  return Response.json(
    {
      error: error instanceof Error ? error.message : String(error),
    },
    { status, headers: dynamicContentHeaders() },
  );
}

export async function readJsonBody<T>(request: Request): Promise<T> {
  return (await request.json()) as T;
}
