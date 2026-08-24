import { describe, expect, test } from 'vitest';
import {
  createClientGuides,
  resolveMcpEndpoint,
} from './mcp-installation-guide';

describe('MCP installation guide', () => {
  test('resolves the MCP endpoint from the current deployment origin', () => {
    expect(resolveMcpEndpoint('https://notes.example.com:8443')).toBe(
      'https://notes.example.com:8443/mcp',
    );
  });

  test('uses the resolved endpoint in every client guide', () => {
    const endpoint = 'https://asterweft.example.com/mcp';
    const guides = createClientGuides(endpoint);

    for (const guide of guides) {
      expect(guide.code).toContain(endpoint);
      expect(guide.code).not.toContain('127.0.0.1');
    }
  });

  test('provides a global Cursor configuration', () => {
    const endpoint = 'https://asterweft.example.com/mcp';
    const cursorGuide = createClientGuides(endpoint).find(
      (guide) => guide.id === 'cursor',
    );

    expect(cursorGuide?.codeLabel).toBe('~/.cursor/mcp.json');
    expect(cursorGuide?.verification).toBe(
      '运行 cursor-agent mcp list 检查连接状态。',
    );
    expect(JSON.parse(cursorGuide?.code ?? '')).toEqual({
      mcpServers: {
        asterweft: {
          url: endpoint,
        },
      },
    });
  });
});
