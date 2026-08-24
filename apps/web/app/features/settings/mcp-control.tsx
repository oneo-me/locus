import { Switch } from '@locus/ui/components/switch';
import { useGlobalSettings } from '../settings/global-settings-provider';

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
