import {
  BotIcon,
  FileTextIcon,
  NetworkIcon,
  PaletteIcon,
  type LucideIcon,
  SettingsIcon,
} from 'lucide-react';
import type { ReactNode } from 'react';
import { NavLink } from 'react-router';
import { buttonVariants } from '@locus/ui/components/button';
import { cn } from '@locus/ui/utils';

interface AppNavigationItem {
  label: string;
  path: string;
  icon: LucideIcon;
}

const APP_NAVIGATION: AppNavigationItem[] = [
  { label: '笔记', path: '/notes', icon: FileTextIcon },
  { label: 'Agent', path: '/agents', icon: BotIcon },
  { label: '知识图谱', path: '/graph', icon: NetworkIcon },
  { label: '设计系统', path: '/design-system', icon: PaletteIcon },
  { label: '设置', path: '/settings', icon: SettingsIcon },
];

export function AppShell({ children }: { children: ReactNode }) {
  return (
    <div className="grid h-dvh min-w-0 grid-cols-[3.5rem_minmax(0,1fr)] bg-background text-sm">
      <aside
        aria-label="应用导航"
        className="flex min-h-0 flex-col items-center border-r border-sidebar-border bg-sidebar py-3 text-sidebar-foreground"
      >
        <div
          className="relative mb-4 shrink-0"
          aria-label="Locus 在线"
          role="status"
        >
          <span
            aria-hidden="true"
            className="block size-9 overflow-hidden rounded-full bg-card shadow-xs"
          >
            <img
              alt=""
              className="size-full object-cover"
              draggable={false}
              height={64}
              src="/icons/locus-mark-64.png"
              width={64}
            />
          </span>
          <span
            aria-hidden="true"
            className="absolute top-0 right-0 size-2.5 rounded-full border-2 border-sidebar bg-success"
          />
        </div>
        <nav aria-label="主要页面" className="flex flex-col gap-1">
          {APP_NAVIGATION.map(({ icon: Icon, label, path }) => (
            <NavLink
              aria-label={label}
              className={({ isActive }) =>
                cn(
                  buttonVariants({
                    size: 'icon',
                    variant: isActive ? 'selected' : 'ghost',
                  }),
                  'cursor-default',
                )
              }
              key={path}
              title={label}
              to={path}
            >
              <Icon aria-hidden="true" />
            </NavLink>
          ))}
        </nav>
      </aside>
      <div className="h-full min-h-0 min-w-0">{children}</div>
    </div>
  );
}
