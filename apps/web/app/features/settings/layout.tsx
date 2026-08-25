import {
  ArchiveIcon,
  InfoIcon,
  LibraryIcon,
  type LucideIcon,
  PlugZapIcon,
  SettingsIcon,
} from 'lucide-react';
import { NavLink, Outlet } from 'react-router';
import { buttonVariants } from '@locus/ui/components/button';
import { cn } from '@locus/ui/utils';

interface SettingsNavItem {
  label: string;
  path: string;
  icon: LucideIcon;
}

interface SettingsNavGroup {
  label: string;
  items: SettingsNavItem[];
}

const SETTINGS_NAV_GROUPS: SettingsNavGroup[] = [
  {
    label: '通用',
    items: [{ label: '常规', path: '/settings/general', icon: SettingsIcon }],
  },
  {
    label: '笔记库',
    items: [
      { label: '配置', path: '/settings/library', icon: LibraryIcon },
      { label: '归档', path: '/settings/archives', icon: ArchiveIcon },
    ],
  },
  {
    label: '连接',
    items: [{ label: 'MCP', path: '/settings/mcp', icon: PlugZapIcon }],
  },
  {
    label: '帮助',
    items: [{ label: '关于', path: '/settings/about', icon: InfoIcon }],
  },
];

export function SettingsLayout() {
  return (
    <div className="grid h-full min-h-0 grid-cols-[14rem_minmax(0,1fr)] overflow-hidden">
      <aside
        aria-label="设置分类"
        className="flex min-h-0 flex-col overflow-hidden border-r bg-sidebar text-sidebar-foreground"
      >
        <header className="flex h-11 shrink-0 items-center border-b border-sidebar-border px-4">
          <h1 className="text-sm font-medium">设置</h1>
        </header>
        <nav className="flex min-h-0 flex-1 flex-col gap-5 overflow-x-hidden overflow-y-auto overscroll-contain p-3">
          {SETTINGS_NAV_GROUPS.map((group) => (
            <section
              aria-labelledby={`settings-group-${group.label}`}
              className="flex flex-col gap-1.5"
              key={group.label}
            >
              <h2
                className="px-2 text-xs font-medium text-sidebar-foreground/50"
                id={`settings-group-${group.label}`}
              >
                {group.label}
              </h2>
              <ul className="flex flex-col gap-1">
                {group.items.map(({ icon: Icon, label, path }) => (
                  <li key={path}>
                    <NavLink
                      className={({ isActive }) =>
                        cn(
                          buttonVariants({
                            size: 'sm',
                            variant: isActive ? 'selected' : 'ghost',
                          }),
                          'w-full cursor-default justify-start',
                        )
                      }
                      end
                      to={path}
                    >
                      <Icon aria-hidden="true" data-icon="inline-start" />
                      {label}
                    </NavLink>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </nav>
      </aside>
      <Outlet />
    </div>
  );
}
