import { type FormEvent, useState } from 'react';
import { useLocation, useNavigate } from 'react-router';
import { type Theme, useTheme } from '@locus/ui/theme-provider';
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
} from '@locus/ui/components/field';
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
} from '@locus/ui/components/input-group';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@locus/ui/components/select';
import { Spinner } from '@locus/ui/components/spinner';
import { useGlobalSettings } from './global-settings-provider';
import { SettingsPage } from './page';
import { SettingsRow, SettingsSection } from './section';

const THEME_OPTIONS: Array<{ value: Theme; label: string }> = [
  { value: 'system', label: '跟随系统' },
  { value: 'light', label: '亮色' },
  { value: 'dark', label: '暗色' },
];

export function GeneralSettings() {
  const location = useLocation();
  const libraryError =
    location.state !== null &&
    typeof location.state === 'object' &&
    'libraryError' in location.state &&
    typeof location.state.libraryError === 'string'
      ? location.state.libraryError
      : undefined;

  return (
    <SettingsPage title="常规">
      <div className="flex flex-col gap-6">
        <LibrarySettings initialError={libraryError} />
        <AppearanceSettings />
      </div>
    </SettingsPage>
  );
}

function LibrarySettings({ initialError }: { initialError?: string }) {
  const { settings, updateSettings } = useGlobalSettings();
  const navigate = useNavigate();
  const [path, setPath] = useState(settings.libraryPath ?? '');
  const [error, setError] = useState<string | null>(initialError ?? null);
  const [saving, setSaving] = useState(false);

  async function saveLibraryPath(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const libraryPath = path.trim();
    if (!libraryPath) {
      setError('请输入笔记库的绝对路径。');
      return;
    }

    setError(null);
    setSaving(true);
    try {
      await updateSettings({ libraryPath });
      await navigate('/notes');
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      setSaving(false);
    }
  }

  return (
    <form onSubmit={saveLibraryPath}>
      <SettingsSection title="笔记库">
        <FieldGroup>
          <Field data-invalid={Boolean(error)} orientation="horizontal">
            <FieldContent>
              <FieldLabel className="font-normal" htmlFor="library-path">
                笔记库路径
              </FieldLabel>
              <FieldDescription className="text-xs">
                包含 Markdown 笔记的本地目录，保存后会重新载入工作区。
              </FieldDescription>
            </FieldContent>
            <div className="flex w-[28rem] min-w-0 flex-col gap-1.5">
              <InputGroup className="bg-card!">
                <InputGroupInput
                  aria-invalid={Boolean(error)}
                  autoComplete="off"
                  id="library-path"
                  onChange={(event) => setPath(event.target.value)}
                  placeholder="/absolute/path/to/notes"
                  spellCheck={false}
                  value={path}
                />
                <InputGroupAddon align="inline-end">
                  <InputGroupButton
                    disabled={
                      saving ||
                      !path.trim() ||
                      path.trim() === settings.libraryPath
                    }
                    type="submit"
                  >
                    {saving ? <Spinner data-icon="inline-start" /> : null}
                    确定
                  </InputGroupButton>
                </InputGroupAddon>
              </InputGroup>
              <FieldError className="text-xs">{error}</FieldError>
            </div>
          </Field>
        </FieldGroup>
      </SettingsSection>
    </form>
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
