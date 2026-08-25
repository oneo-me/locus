import FluentIconEye from '@oneo/fluentui-icons-like-react/FluentIconEye';
import FluentIconInfo from '@oneo/fluentui-icons-like-react/FluentIconInfo';
import FluentIconPaintBrush from '@oneo/fluentui-icons-like-react/FluentIconPaintBrush';
import FluentIconPlugConnected from '@oneo/fluentui-icons-like-react/FluentIconPlugConnected';
import FluentIconSearch from '@oneo/fluentui-icons-like-react/FluentIconSearch';
import { FolderOpenIcon } from 'lucide-react';
import { useNavigate } from 'react-router';
import {
  Children,
  type ComponentType,
  type FormEvent,
  Fragment,
  isValidElement,
  type ReactNode,
  useMemo,
  useState,
} from 'react';
import { type Theme, useTheme } from '@locus/ui/theme-provider';
import { Badge } from '@locus/ui/components/badge';
import { Button } from '@locus/ui/components/button';
import {
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
} from '@locus/ui/components/field';
import { Input } from '@locus/ui/components/input';
import {
  InputGroup,
  InputGroupAddon,
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
import { cn } from '@locus/ui/utils';

type SettingsSectionId = 'library' | 'appearance' | 'mcp' | 'about';

interface SettingsNavItem {
  id: SettingsSectionId;
  label: string;
  description: string;
  group: string;
  keywords: string[];
  icon: ComponentType<{
    'aria-hidden'?: boolean | 'true' | 'false';
    'data-icon'?: string;
    size?: number;
    title?: null;
  }>;
}

const SETTINGS_NAV: SettingsNavItem[] = [
  {
    id: 'library',
    label: '文档库',
    description: '选择 Locus 使用的本地 Markdown 文档库。',
    group: '工作区',
    keywords: ['文档库', '路径', '文件夹', 'Markdown'],
    icon: FolderOpenIcon,
  },
  {
    id: 'appearance',
    label: '外观',
    description: '控制主题与界面呈现。更改后立即生效。',
    group: '偏好',
    keywords: ['主题', '亮色', '暗色', '跟随系统'],
    icon: FluentIconPaintBrush,
  },
  {
    id: 'mcp',
    label: 'MCP',
    description: '允许 MCP 客户端通过 Streamable HTTP 访问文档库。',
    group: '服务',
    keywords: ['MCP', '协议', 'Streamable HTTP', '文章'],
    icon: FluentIconPlugConnected,
  },
  {
    id: 'about',
    label: '关于',
    description: '当前软件版本与运行环境。',
    group: '关于',
    keywords: ['版本', 'Locus'],
    icon: FluentIconInfo,
  },
];

export function SettingsBrowser({
  initialSection = 'appearance',
  libraryError,
}: {
  initialSection?: SettingsSectionId;
  libraryError?: string;
}) {
  const [activeId, setActiveId] = useState<SettingsSectionId>(initialSection);
  const [query, setQuery] = useState('');

  const filteredNav = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    if (!normalized) return SETTINGS_NAV;
    return SETTINGS_NAV.filter((item) => {
      const haystack = [item.label, item.description, ...item.keywords]
        .join(' ')
        .toLowerCase();
      return haystack.includes(normalized);
    });
  }, [query]);

  const navGroups = useMemo(() => {
    const groups: Array<{ label: string; items: SettingsNavItem[] }> = [];
    for (const item of filteredNav) {
      const last = groups.at(-1);
      if (last?.label === item.group) {
        last.items.push(item);
      } else {
        groups.push({ label: item.group, items: [item] });
      }
    }
    return groups;
  }, [filteredNav]);

  const activeItem =
    filteredNav.find((item) => item.id === activeId) ?? filteredNav[0] ?? null;

  return (
    <div className="grid h-full min-h-0 grid-cols-[13rem_minmax(0,1fr)] overflow-hidden">
      <SettingsSidebar
        activeId={activeItem?.id ?? null}
        groups={navGroups}
        onSelect={setActiveId}
        onQueryChange={setQuery}
        query={query}
      />
      <div className="flex min-h-0 min-w-0 flex-col bg-card">
        <header className="flex h-12 shrink-0 items-center px-6">
          <h1 className="truncate text-sm font-medium">
            {activeItem?.label ?? '设置'}
          </h1>
        </header>
        <div className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto overscroll-contain">
          {activeItem ? (
            <div className="px-6 pt-3 pb-8">
              <SettingsPanel id={activeItem.id} libraryError={libraryError} />
            </div>
          ) : (
            <div className="flex h-full items-center justify-center px-6">
              <p className="text-sm text-muted-foreground">
                没有匹配的设置项。
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function SettingsSidebar({
  activeId,
  groups,
  onSelect,
  onQueryChange,
  query,
}: {
  activeId: SettingsSectionId | null;
  groups: Array<{ label: string; items: SettingsNavItem[] }>;
  onSelect: (id: SettingsSectionId) => void;
  onQueryChange: (value: string) => void;
  query: string;
}) {
  return (
    <aside
      className="flex min-h-0 flex-col overflow-hidden border-r bg-sidebar text-sidebar-foreground"
      aria-label="设置分类"
    >
      <header className="flex h-12 shrink-0 items-center px-3">
        <h2 className="text-sm font-medium">设置</h2>
      </header>
      <div className="flex min-h-0 flex-1 flex-col gap-2.5 px-3 pb-3">
        <InputGroup className="h-8 bg-card [&_svg:not([class*='size-'])]:size-4">
          <InputGroupAddon>
            <FluentIconSearch aria-hidden="true" size={16} title={null} />
          </InputGroupAddon>
          <InputGroupInput
            aria-label="搜索设置"
            autoComplete="off"
            onChange={(event) => onQueryChange(event.target.value)}
            placeholder="搜索设置…"
            value={query}
          />
        </InputGroup>
        <nav className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto overscroll-contain">
          {groups.length > 0 ? (
            <div className="flex flex-col gap-2">
              {groups.map((group) => (
                <div key={group.label} className="flex flex-col gap-1">
                  <p className="px-2 text-xs text-muted-foreground">
                    {group.label}
                  </p>
                  <ul className="flex flex-col gap-px">
                    {group.items.map((item) => {
                      const Icon = item.icon;
                      const active = item.id === activeId;
                      return (
                        <li key={item.id}>
                          <button
                            aria-current={active ? 'true' : undefined}
                            className={cn(
                              'flex h-7 w-full cursor-default items-center gap-1.5 rounded-lg px-2 text-left text-sm outline-none transition-colors focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 [&_svg]:size-3.5',
                              active
                                ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                                : 'text-muted-foreground hover:bg-muted/60 hover:text-foreground',
                            )}
                            onClick={() => onSelect(item.id)}
                            type="button"
                          >
                            <Icon
                              aria-hidden="true"
                              data-icon="inline-start"
                              size={16}
                              title={null}
                            />
                            {item.label}
                          </button>
                        </li>
                      );
                    })}
                  </ul>
                </div>
              ))}
            </div>
          ) : (
            <p className="px-2 text-xs text-muted-foreground">无匹配分类</p>
          )}
        </nav>
        <a
          aria-label="设计系统"
          className="flex h-7 shrink-0 cursor-default items-center gap-1.5 rounded-lg px-2 text-sm text-muted-foreground outline-none transition-colors hover:bg-muted/60 hover:text-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 [&_svg]:size-3.5"
          href="/design-system"
          rel="noopener noreferrer"
          target="_blank"
        >
          <FluentIconEye
            aria-hidden="true"
            data-icon="inline-start"
            size={16}
            title={null}
          />
          设计系统
        </a>
      </div>
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
    case 'library':
      return <LibraryPanel initialError={libraryError} />;
    case 'appearance':
      return <AppearancePanel />;
    case 'mcp':
      return <McpPanel />;
    case 'about':
      return <AboutPanel />;
  }
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
      setError('请输入文档库的绝对路径。');
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
      <SettingsBlock title="本地文档库">
        <FieldGroup>
          <Field data-invalid={Boolean(error)}>
            <FieldLabel htmlFor="library-path">文档库路径</FieldLabel>
            <Input
              aria-invalid={Boolean(error)}
              autoComplete="off"
              id="library-path"
              onChange={(event) => setPath(event.target.value)}
              placeholder="/absolute/path/to/markdown-library"
              spellCheck={false}
              value={path}
            />
            <FieldDescription>
              填写包含 Markdown 文章目录的绝对路径。保存后会重新载入工作区。
            </FieldDescription>
            <FieldError>{error}</FieldError>
          </Field>
          <div className="flex justify-end">
            <Button
              disabled={
                saving || !path.trim() || path.trim() === settings.libraryPath
              }
              type="submit"
            >
              {saving ? <Spinner data-icon="inline-start" /> : null}
              保存并载入
            </Button>
          </div>
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
