import FluentIconSettings from '@oneo/fluentui-icons-like-react/FluentIconSettings';
import { lazy, Suspense, useEffect, useState } from 'react';
import { createSearchParams, Link } from 'react-router';
import { Button } from '@locus/ui/components/button';
import { workspaceMobilePrimaryState } from './mobile-navigation';

const SettingsDialog = lazy(() =>
  import('../settings/browser').then((module) => ({
    default: module.SettingsDialog,
  })),
);

export type WorkspaceEventConnection =
  | 'connecting'
  | 'connected'
  | 'reconnecting';

export interface WorkspaceEventStatus {
  connection: WorkspaceEventConnection;
  lastEventAt: string | null;
}

export function getWorkspaceEventStatusText(
  { connection, lastEventAt }: WorkspaceEventStatus,
  refreshing = false,
): string {
  if (connection === 'connecting') return '正在连接实时事件…';
  if (connection === 'reconnecting') return '连接中断，正在重试…';
  if (refreshing) return '正在刷新文档库…';
  if (!lastEventAt) return '正在监听文档事件';
  return lastEventAt;
}

export function WorkspaceBrandMark() {
  return (
    <span
      aria-hidden="true"
      className="block size-9 overflow-hidden rounded-full bg-card shadow-xs"
    >
      <img
        alt=""
        className="size-full object-cover"
        draggable={false}
        height={64}
        src="/icons/asterweft-mark-64.png"
        width={64}
      />
    </span>
  );
}

function SettingsControl() {
  const [hydrated, setHydrated] = useState(false);
  const [requested, setRequested] = useState(false);

  useEffect(() => {
    setHydrated(true);
  }, []);

  if (!requested) {
    return (
      <Button
        aria-label="设置"
        className="cursor-default"
        disabled={!hydrated}
        onClick={() => setRequested(true)}
        size="icon"
        type="button"
        variant="ghost"
      >
        <FluentIconSettings
          aria-hidden="true"
          data-icon="inline-start"
          title={null}
        />
      </Button>
    );
  }

  return (
    <Suspense
      fallback={
        <Button
          aria-busy="true"
          aria-label="正在加载设置"
          className="cursor-default"
          disabled
          size="icon"
          type="button"
          variant="ghost"
        >
          <FluentIconSettings
            aria-hidden="true"
            data-icon="inline-start"
            title={null}
          />
        </Button>
      }
    >
      <SettingsDialog initiallyOpen />
    </Suspense>
  );
}

export function WorkspaceHeader({
  eventStatus,
  query,
  refreshing,
}: {
  eventStatus: WorkspaceEventStatus;
  query?: string;
  refreshing: boolean;
}) {
  const connected = eventStatus.connection === 'connected';

  return (
    <header
      aria-busy={refreshing}
      className="shrink-0 px-3 pt-3 pb-2 text-sm font-normal"
    >
      <div className="grid min-w-0 grid-cols-[minmax(0,1fr)_auto] items-center gap-2">
        <Link
          aria-label="Asterweft 首页"
          className="flex min-w-0 cursor-default items-center gap-2.5 rounded-lg pr-1 no-underline outline-none transition-colors hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50"
          state={workspaceMobilePrimaryState}
          to={{
            pathname: '/',
            search: query ? `?${createSearchParams({ q: query })}` : '',
          }}
        >
          <div className="relative shrink-0">
            <WorkspaceBrandMark />
            {connected ? (
              <span
                className="absolute top-0 right-0 size-2.5 rounded-full border-2 border-sidebar bg-success"
                aria-hidden="true"
              />
            ) : null}
          </div>
          <div className="min-w-0 flex-1">
            <span className="block truncate text-sm leading-5 font-medium text-foreground">
              Asterweft
            </span>
            <div
              className="truncate text-xs leading-4 font-normal text-muted-foreground"
              role="status"
            >
              <span className="sr-only">
                {connected ? '事件流已连接。' : null}
              </span>
              {getWorkspaceEventStatusText(eventStatus, refreshing)}
            </div>
          </div>
        </Link>
        <div className="flex shrink-0 items-center gap-1">
          <SettingsControl />
        </div>
      </div>
    </header>
  );
}
