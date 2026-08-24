import { DEFAULT_GLOBAL_SETTINGS } from '~/server/features/settings/global-settings-types';
import { describe, expect, test } from 'vitest';
import { createAsterweftMcpRequestHandler } from './handler.server';

describe('MCP HTTP handler', () => {
  test('returns 404 without opening the library while MCP is disabled', async () => {
    let opened = false;
    const handler = createAsterweftMcpRequestHandler({
      getLibrary: async () => {
        opened = true;
        throw new Error('不应打开文档库');
      },
      readGlobalSettings: () => ({
        ...DEFAULT_GLOBAL_SETTINGS,
        mcpEnabled: false,
      }),
    });

    const response = await handler(
      new Request('http://127.0.0.1:3001/mcp', { method: 'POST' }),
    );
    expect(response.status).toBe(404);
    expect(opened).toBe(false);
  });
});
