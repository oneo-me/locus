import { describe, expect, test } from 'vitest';
import {
  e2eLibraryPath,
  requireE2ELibraryPath,
  requireIsolatedE2EPort,
} from './e2e-web-server-config';

describe('E2E Web 服务隔离', () => {
  test('接受显式的独立端口', () => {
    expect(requireIsolatedE2EPort(['--port', '4179'])).toBe(4179);
    expect(requireIsolatedE2EPort(['--port=4180'])).toBe(4180);
  });

  test('拒绝开发端口、缺失端口和无效端口', () => {
    expect(() => requireIsolatedE2EPort(['--port', '3000'])).toThrow(
      '禁止使用开发服务器端口 3000',
    );
    expect(() => requireIsolatedE2EPort([])).toThrow('必须显式使用');
    expect(() => requireIsolatedE2EPort(['--port', '0'])).toThrow(
      '必须显式使用',
    );
  });

  test('只允许测试 fixture 文档库', () => {
    expect(requireE2ELibraryPath(e2eLibraryPath)).toBe(e2eLibraryPath);
    expect(() => requireE2ELibraryPath('/tmp/real-library')).toThrow(
      '只能使用 tests/fixtures/library',
    );
  });
});
