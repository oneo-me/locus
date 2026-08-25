import {
  InfoIcon,
  type LucideIcon,
  PlugZapIcon,
  SettingsIcon,
} from 'lucide-react';
import { useNavigate } from 'react-router';
import {
  Children,
  type FormEvent,
  Fragment,
  isValidElement,
  type ReactNode,
  useState,
} from 'react';
import { type Theme, useTheme } from '@locus/ui/theme-provider';
import { Badge } from '@locus/ui/components/badge';
import { Button } from '@locus/ui/components/button';
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
import { Separator } from '@locus/ui/components/separator';
import { Spinner } from '@locus/ui/components/spinner';
import { McpEnabledSwitch } from '../settings/mcp-control';
import { McpInstallationGuide } from '../settings/mcp-installation-guide';
import { useGlobalSettings } from '../settings/global-settings-provider';

type SettingsSectionId = 'general' | 'mcp' | 'about';

interface SettingsNavItem {
  id: SettingsSectionId;
  label: string;
  icon: LucideIcon;
}

const SETTINGS_NAV: SettingsNavItem[] = [
  {
    id: 'general',
    label: '常规',
    icon: SettingsIcon,
  },
  {
    id: 'mcp',
    label: 'MCP',
    icon: PlugZapIcon,
  },
  {
    id: 'about',
    label: '关于',
    icon: InfoIcon,
  },
];

export function SettingsBrowser({ libraryError }: { libraryError?: string }) {
  const [activeId, setActiveId] = useState<SettingsSectionId>('general');
  const activeItem =
    SETTINGS_NAV.find((item) => item.id === activeId) ?? SETTINGS_NAV[0];

  return (
    <div className="grid h-full min-h-0 grid-cols-[14rem_minmax(0,1fr)] overflow-hidden">
      <SettingsSidebar activeId={activeItem.id} onSelect={setActiveId} />
      <div className="flex min-h-0 min-w-0 flex-col bg-card">
        <header className="flex h-11 shrink-0 items-center border-b px-5">
          <h1 className="truncate text-sm font-medium">{activeItem.label}</h1>
        </header>
        <div className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto overscroll-contain">
          <div className="px-5 pt-5 pb-8">
            <SettingsPanel id={activeItem.id} libraryError={libraryError} />
          </div>
        </div>
      </div>
    </div>
  );
}

function SettingsSidebar({
  activeId,
  onSelect,
}: {
  activeId: SettingsSectionId;
  onSelect: (id: SettingsSectionId) => void;
}) {
  return (
    <aside
      className="flex min-h-0 flex-col overflow-hidden border-r bg-sidebar text-sidebar-foreground"
      aria-label="设置分类"
    >
      <header className="flex h-11 shrink-0 items-center border-b border-sidebar-border px-4">
        <h2 className="text-sm font-medium">设置</h2>
      </header>
      <nav className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto overscroll-contain p-3">
        <ul className="flex flex-col gap-1">
          {SETTINGS_NAV.map((item) => {
            const Icon = item.icon;
            const active = item.id === activeId;
            return (
              <li key={item.id}>
                <Button
                  aria-current={active ? 'true' : undefined}
                  className="w-full cursor-default justify-start"
                  onClick={() => onSelect(item.id)}
                  size="sm"
                  type="button"
                  variant={active ? 'selected' : 'ghost'}
                >
                  <Icon aria-hidden="true" data-icon="inline-start" />
                  {item.label}
                </Button>
              </li>
            );
          })}
        </ul>
      </nav>
    </aside>
  );
}

function SettingsPanel({
  id,
  libraryError,
}: {
  id: SettingsSectionId;
  libraryError?: string;
}) {
  switch (id) {
    case 'general':
      return <GeneralPanel libraryError={libraryError} />;
    case 'mcp':
      return <McpPanel />;
    case 'about':
      return <AboutPanel />;
  }
}

function GeneralPanel({ libraryError }: { libraryError?: string }) {
  return (
    <div className="flex flex-col gap-6">
      <LibraryPanel initialError={libraryError} />
      <AppearancePanel />
    </div>
  );
}

function LibraryPanel({ initialError }: { initialError?: string }) {
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
      <SettingsBlock title="笔记库">
        <FieldGroup>
          <Field data-invalid={Boolean(error)} orientation="horizontal">
            <FieldContent>
              <FieldLabel className="font-normal" htmlFor="library-path">
                笔记路路径
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
      </SettingsBlock>
    </form>
  );
}

function SettingsBlock({
  children,
  title,
}: {
  children: ReactNode;
  title: string;
}) {
  const items = Children.toArray(children);

  return (
    <section className="flex flex-col gap-2">
      <h3 className="px-2 text-sm font-medium">{title}</h3>
      <div className="flex flex-col overflow-hidden rounded-xl bg-foreground/2 p-3">
        {items.map((item, index) => (
          <Fragment
            key={isValidElement(item) && item.key !== null ? item.key : index}
          >
            {index > 0 ? <Separator /> : null}
            {item}
          </Fragment>
        ))}
      </div>
    </section>
  );
}

function SettingsRow({
  children,
  description,
  label,
}: {
  children: ReactNode;
  description?: string;
  label: string;
}) {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 py-3 first:pt-0 last:pb-0">
      <div className="min-w-0">
        <p className="text-sm">{label}</p>
        {description ? (
          <p className="mt-0.5 text-xs text-muted-foreground">{description}</p>
        ) : null}
      </div>
      <div className="flex min-w-0 items-center justify-end gap-2">
        {children}
      </div>
    </div>
  );
}

const THEME_OPTIONS: Array<{ value: Theme; label: string }> = [
  { value: 'system', label: '跟随系统' },
  { value: 'light', label: '亮色' },
  { value: 'dark', label: '暗色' },
];

function AppearancePanel() {
  const { theme, setTheme } = useTheme();

  return (
    <SettingsBlock title="主题">
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
    </SettingsBlock>
  );
}

function McpPanel() {
  return (
    <div className="flex flex-col gap-6">
      <SettingsBlock title="本机服务">
        <SettingsRow
          description="允许 MCP 客户端通过 Streamable HTTP 浏览与搜索文档库；正文与附件由客户端直接编辑本地文件。"
          label="启用 MCP"
        >
          <McpEnabledSwitch />
        </SettingsRow>
      </SettingsBlock>
      <SettingsBlock title="连接应用">
        <McpInstallationGuide />
      </SettingsBlock>
    </div>
  );
}

function AboutPanel() {
  return (
    <div className="flex flex-col gap-6">
      <SettingsBlock title="应用">
        <SettingsRow description="Knowledge, woven together" label="Locus">
          <Badge variant="secondary">v0.1.0</Badge>
        </SettingsRow>
      </SettingsBlock>
    </div>
  );
}
