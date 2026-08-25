import { McpEnabledSwitch } from './mcp-control';
import { McpInstallationGuide } from './mcp-installation-guide';
import { SettingsPage } from './page';
import { SettingsRow, SettingsSection } from './section';

export function McpSettings() {
  return (
    <SettingsPage title="MCP">
      <div className="flex flex-col gap-6">
        <SettingsSection title="本机服务">
          <SettingsRow
            description="允许 MCP 客户端通过 Streamable HTTP 浏览与搜索文档库；正文与附件由客户端直接编辑本地文件。"
            label="启用 MCP"
          >
            <McpEnabledSwitch />
          </SettingsRow>
        </SettingsSection>
        <SettingsSection title="连接应用">
          <McpInstallationGuide />
        </SettingsSection>
      </div>
    </SettingsPage>
  );
}
