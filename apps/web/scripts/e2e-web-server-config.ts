import { resolve } from 'node:path';

export const e2eLibraryPath = resolve(
  import.meta.dirname,
  '../tests/fixtures/library',
);

export function requireIsolatedE2EPort(args: string[]): number {
  const inlinePort = args
    .find((argument) => argument.startsWith('--port='))
    ?.slice('--port='.length);
  const portIndex = args.indexOf('--port');
  const value =
    inlinePort ?? (portIndex >= 0 ? args[portIndex + 1] : undefined);
  const port = value && /^\d+$/.test(value) ? Number.parseInt(value, 10) : NaN;

  if (!Number.isInteger(port) || port < 1024 || port > 65_535) {
    throw new Error('E2E 服务必须显式使用 1024–65535 范围内的独立端口。');
  }
  if (port === 3000) {
    throw new Error('E2E 服务禁止使用开发服务器端口 3000。');
  }

  return port;
}

export function requireE2ELibraryPath(value: string | undefined): string {
  const resolved = value?.trim() ? resolve(value) : '';
  if (resolved !== e2eLibraryPath) {
    throw new Error('E2E 服务只能使用 tests/fixtures/library 文档库。');
  }
  return resolved;
}
