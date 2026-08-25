import type { LibraryNavigationDocument } from '@locus/document-library/contracts/library-api';
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
import { type FormEvent, useState } from 'react';
import { useLocation, useNavigate } from 'react-router';
import { useGlobalSettings } from './global-settings-provider';
import { SettingsPage } from './page';
import { SettingsRow, SettingsSection } from './section';

export function LibrarySettings({
  documents,
  documentsError,
}: {
  documents: LibraryNavigationDocument[];
  documentsError: string | null;
}) {
  const location = useLocation();
  const libraryError =
    location.state !== null &&
    typeof location.state === 'object' &&
    'libraryError' in location.state &&
    typeof location.state.libraryError === 'string'
      ? location.state.libraryError
      : undefined;

  return (
    <SettingsPage title="笔记库">
      <div className="flex flex-col gap-6">
        <LibraryPathSettings initialError={libraryError} />
        <DefaultDocumentSettings
          documents={documents}
          documentsError={documentsError}
        />
      </div>
    </SettingsPage>
  );
}

function LibraryPathSettings({ initialError }: { initialError?: string }) {
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
      <SettingsSection title="位置">
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

function DefaultDocumentSettings({
  documents,
  documentsError,
}: {
  documents: LibraryNavigationDocument[];
  documentsError: string | null;
}) {
  const { settings, updateSettings } = useGlobalSettings();
  const [error, setError] = useState<string | null>(documentsError);
  const [saving, setSaving] = useState(false);
  const options = documents
    .filter((document) => !document.directory.includes('/'))
    .map((document) => ({ label: document.title, value: document.path }));
  const value = options.some(
    (option) => option.value === settings.defaultDocumentPath,
  )
    ? settings.defaultDocumentPath
    : null;

  async function saveDefaultDocument(defaultDocumentPath: string | null) {
    if (!defaultDocumentPath) return;
    setError(null);
    setSaving(true);
    try {
      await updateSettings({ defaultDocumentPath });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  return (
    <SettingsSection title="默认设置">
      <SettingsRow
        description="进入首页或笔记页面时自动打开的顶级笔记。"
        label="默认笔记"
      >
        <Field className="w-64" data-invalid={Boolean(error)}>
          <Select
            disabled={saving || options.length === 0}
            items={options}
            onValueChange={saveDefaultDocument}
            value={value}
          >
            <SelectTrigger
              aria-invalid={Boolean(error)}
              aria-label="默认笔记"
              className="w-full bg-card"
            >
              <SelectValue placeholder="选择顶级笔记" />
            </SelectTrigger>
            <SelectContent alignItemWithTrigger={false}>
              <SelectGroup>
                {options.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
          <FieldError className="max-w-64 text-xs">{error}</FieldError>
        </Field>
      </SettingsRow>
    </SettingsSection>
  );
}
