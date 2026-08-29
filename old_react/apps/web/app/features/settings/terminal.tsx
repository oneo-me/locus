import type { TerminalTheme } from '@locus/document-library/contracts/global-settings';
import { type FormEvent, useState } from 'react';
import { Alert, AlertDescription } from '@locus/ui/components/alert';
import { Button } from '@locus/ui/components/button';
import { Input } from '@locus/ui/components/input';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@locus/ui/components/select';
import { useGlobalSettings } from './global-settings-provider';
import { SettingsPage } from './page';
import { SettingsRow, SettingsSection } from './section';

const TERMINAL_THEME_OPTIONS: Array<{
  label: string;
  value: TerminalTheme;
}> = [
  { label: '自动', value: 'auto' },
  { label: '亮色', value: 'light' },
  { label: '暗色', value: 'dark' },
];

export function TerminalSettings() {
  const { settings, updateSettings } = useGlobalSettings();
  const [shell, setShell] = useState(settings.terminalShell ?? '');
  const [startupCommand, setStartupCommand] = useState(
    settings.terminalStartupCommand,
  );
  const [terminalTheme, setTerminalTheme] = useState(settings.terminalTheme);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);
    setSaved(false);
    setError(null);
    try {
      await updateSettings({
        terminalShell: shell.trim() || null,
        terminalStartupCommand: startupCommand.trim(),
        terminalTheme,
      });
      setSaved(true);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  return (
    <SettingsPage title="终端">
      <form className="flex flex-col gap-4" onSubmit={save}>
        <SettingsSection title="终端配置">
          <SettingsRow
            description="留空时使用运行 Locus 的默认登录 Shell。"
            label="Shell"
          >
            <Input
              autoComplete="off"
              className="w-80 bg-card"
              maxLength={1024}
              onChange={(event) => setShell(event.target.value)}
              placeholder="/bin/zsh"
              spellCheck={false}
              value={shell}
            />
          </SettingsRow>
          <SettingsRow
            description="每个新终端会话建立后自动执行一次。"
            label="启动命令"
          >
            <Input
              autoComplete="off"
              className="w-80 bg-card"
              maxLength={4096}
              onChange={(event) =>
                setStartupCommand(event.target.value.replace(/[\r\n]/g, ''))
              }
              placeholder="例如 pnpm dev"
              spellCheck={false}
              value={startupCommand}
            />
          </SettingsRow>
          <SettingsRow
            description="自动模式跟随 Locus 当前的亮色或暗色主题。"
            label="主题配色"
          >
            <Select
              items={TERMINAL_THEME_OPTIONS}
              onValueChange={(value) => {
                if (value === 'auto' || value === 'light' || value === 'dark') {
                  setTerminalTheme(value);
                }
              }}
              value={terminalTheme}
            >
              <SelectTrigger aria-label="终端主题配色" className="w-36 bg-card">
                <SelectValue />
              </SelectTrigger>
              <SelectContent alignItemWithTrigger={false}>
                <SelectGroup>
                  {TERMINAL_THEME_OPTIONS.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </SettingsRow>
        </SettingsSection>
        {error ? (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        ) : null}
        <div className="flex items-center justify-end gap-3">
          {saved ? (
            <span className="text-xs text-muted-foreground" role="status">
              已保存
            </span>
          ) : null}
          <Button disabled={saving} type="submit">
            {saving ? '正在保存…' : '保存'}
          </Button>
        </div>
      </form>
    </SettingsPage>
  );
}
