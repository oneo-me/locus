import { Switch } from '~/components/ui/switch';
import { useGlobalSettings } from '~/features/settings/global-settings-provider';

export function McpEnabledSwitch() {
  const { settings, updateSettings } = useGlobalSettings();

  return (
    <Switch
      aria-label="启用 MCP"
      checked={settings.mcpEnabled}
      onCheckedChange={(checked) => {
        void updateSettings({ mcpEnabled: checked }).catch(() => {});
      }}
    />
  );
}
