import { type Theme, useTheme } from '@locus/ui/theme-provider';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@locus/ui/components/select';
import { Switch } from '@locus/ui/components/switch';
import { useGlobalSettings } from './global-settings-provider';
import { SettingsPage } from './page';
import { SettingsRow, SettingsSection } from './section';

const THEME_OPTIONS: Array<{ value: Theme; label: string }> = [
  { value: 'system', label: '跟随系统' },
  { value: 'light', label: '亮色' },
  { value: 'dark', label: '暗色' },
];

export function GeneralSettings() {
  return (
    <SettingsPage title="常规">
      <div className="flex flex-col gap-6">
        <AppearanceSettings />
        <CommandPaletteSettings />
      </div>
    </SettingsPage>
  );
}

function CommandPaletteSettings() {
  const { settings, updateSettings } = useGlobalSettings();

  return (
    <SettingsSection title="命令面板">
      <SettingsRow
        description="打开命令面板时直接显示命令和笔记。"
        label="默认展开"
      >
        <Switch
          aria-label="默认展开命令面板"
          checked={settings.commandPaletteDefaultExpanded}
          onCheckedChange={(checked) => {
            void updateSettings({
              commandPaletteDefaultExpanded: checked,
            }).catch(() => {});
          }}
        />
      </SettingsRow>
    </SettingsSection>
  );
}

function AppearanceSettings() {
  const { theme, setTheme } = useTheme();

  return (
    <SettingsSection title="主题">
      <SettingsRow
        description="默认跟随系统，也可固定为亮色或暗色。"
        label="外观模式"
      >
        <Select
          items={THEME_OPTIONS}
          onValueChange={(value) => {
            if (value === 'light' || value === 'dark' || value === 'system') {
              setTheme(value);
            }
          }}
          value={theme}
        >
          <SelectTrigger aria-label="外观模式" className="w-36 bg-card">
            <SelectValue />
          </SelectTrigger>
          <SelectContent alignItemWithTrigger={false}>
            <SelectGroup>
              {THEME_OPTIONS.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
      </SettingsRow>
    </SettingsSection>
  );
}
