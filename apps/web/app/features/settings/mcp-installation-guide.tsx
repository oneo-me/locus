import { useEffect, useState } from 'react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '~/components/ui/tabs';

const MCP_PATH = '/mcp';

export function resolveMcpEndpoint(origin: string) {
  return new URL(MCP_PATH, origin).toString();
}

export function createClientGuides(mcpEndpoint: string) {
  return [
    {
      id: 'codex',
      label: 'Codex',
      description: '在终端运行命令。Codex CLI 与 IDE 扩展会共享这项配置。',
      codeLabel: '终端',
      code: `codex mcp add asterweft --url ${mcpEndpoint}`,
      verification: '进入 Codex 后运行 /mcp 检查连接。',
    },
    {
      id: 'claude-code',
      label: 'Claude Code',
      description: '在终端添加一个 HTTP 类型的 MCP 服务。',
      codeLabel: '终端',
      code: `claude mcp add --transport http asterweft ${mcpEndpoint}`,
      verification: '进入 Claude Code 后运行 /mcp 检查连接。',
    },
    {
      id: 'cursor',
      label: 'Cursor',
      description: '将远程 MCP 服务添加到 Cursor IDE 与 Agent 共用的全局配置。',
      codeLabel: '~/.cursor/mcp.json',
      code: `{
  "mcpServers": {
    "asterweft": {
      "url": "${mcpEndpoint}"
    }
  }
}`,
      verification: '运行 cursor-agent mcp list 检查连接状态。',
    },
    {
      id: 'opencode',
      label: 'OpenCode',
      description: '在终端添加一个远程 MCP 服务。',
      codeLabel: '终端',
      code: `opencode mcp add asterweft --url ${mcpEndpoint}`,
      verification: '运行 opencode mcp list 检查连接状态。',
    },
    {
      id: 'other',
      label: '其他客户端',
      description: '在支持 MCP 的客户端中手动添加远程服务。',
      codeLabel: '连接参数',
      code: `Name: asterweft
Transport: Streamable HTTP
URL: ${mcpEndpoint}
Authentication: none`,
      verification: '不同客户端的字段名称可能略有差异。',
    },
  ] as const;
}

export function McpInstallationGuide() {
  const [mcpEndpoint, setMcpEndpoint] = useState(MCP_PATH);

  useEffect(() => {
    setMcpEndpoint(resolveMcpEndpoint(window.location.origin));
  }, []);

  const clientGuides = createClientGuides(mcpEndpoint);

  return (
    <Tabs defaultValue="codex">
      <TabsList
        aria-label="MCP 客户端安装方式"
        className="max-w-full justify-start overflow-x-auto"
        variant="line"
      >
        {clientGuides.map((guide) => (
          <TabsTrigger
            className="shrink-0 px-2"
            key={guide.id}
            value={guide.id}
          >
            {guide.label}
          </TabsTrigger>
        ))}
      </TabsList>
      {clientGuides.map((guide) => (
        <TabsContent className="min-w-0 pt-3" key={guide.id} value={guide.id}>
          <p className="text-xs text-muted-foreground">{guide.description}</p>
          <div className="mt-3 min-w-0 overflow-hidden rounded-lg border bg-card">
            <div className="border-b px-3 py-1.5 text-xs text-muted-foreground">
              {guide.codeLabel}
            </div>
            <pre className="max-w-full overflow-x-auto px-3 py-2.5 text-xs leading-5">
              <code>{guide.code}</code>
            </pre>
          </div>
          <p className="mt-2 text-xs text-muted-foreground">
            {guide.verification}
          </p>
        </TabsContent>
      ))}
    </Tabs>
  );
}
