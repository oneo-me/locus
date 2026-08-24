import type { LibraryWorkspaceView } from '@locus/document-library/contracts/library-api';
import type { CSSProperties } from 'react';
import {
  createContext,
  type ReactNode,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react';
import { cn } from '@locus/ui/utils';
import {
  parseWorkspaceEventSnapshot,
  type WorkspaceEventSnapshot,
  WORKSPACE_EVENTS_PATH,
} from './events';
import { useGlobalSettings } from '../settings/global-settings-provider';
import type { WorkspaceEventStatus } from './header';
import { WorkspaceMobileEdgeSwipe } from './mobile-navigation';
import { WorkspaceSidebar } from './sidebar';
import { WorkspaceSidebarResizeHandle } from './sidebar-resize-handle';

export interface WorkspaceSnapshotState {
  query: string | null;
  snapshot: WorkspaceEventSnapshot;
}

function liveSnapshotMatchesLoaderData(
  data: LibraryWorkspaceView,
  snapshot: WorkspaceEventSnapshot,
): boolean {
  return (
    JSON.stringify(data.status) === JSON.stringify(snapshot.status) &&
    JSON.stringify(data.directories) ===
      JSON.stringify(snapshot.navigation.directories) &&
    JSON.stringify(data.documents) ===
      JSON.stringify(snapshot.navigation.documents) &&
    JSON.stringify(data.search) ===
      JSON.stringify(snapshot.navigation.search) &&
    JSON.stringify(data.repositoryChanges) ===
      JSON.stringify(snapshot.repositoryChanges) &&
    data.repositoryError === snapshot.repositoryError &&
    data.repositoryRevision === snapshot.repositoryRevision
  );
}

const WorkspaceLibraryDataContext = createContext<LibraryWorkspaceView | null>(
  null,
);

const eventTimeFormatter = new Intl.DateTimeFormat('zh-CN', {
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  hour12: false,
});

function createInitialEventStatus(): WorkspaceEventStatus {
  return {
    connection: 'connecting',
    lastEventAt: null,
  };
}

export function createWorkspaceEventPath(query: string | null): string {
  const search = new URLSearchParams();
  if (query) search.set('q', query);
  return search.size > 0
    ? `${WORKSPACE_EVENTS_PATH}?${search}`
    : WORKSPACE_EVENTS_PATH;
}

export function mergeWorkspaceLibraryData(
  data: LibraryWorkspaceView,
  snapshotState: WorkspaceSnapshotState | null,
  query: string | null,
): LibraryWorkspaceView {
  const snapshot = snapshotState?.snapshot;
  if (snapshot && liveSnapshotMatchesLoaderData(data, snapshot)) {
    return data;
  }
  const searchSnapshot = snapshotState?.query === query ? snapshot : undefined;
  return {
    ...data,
    ...(snapshot
      ? {
          directories: snapshot.navigation.directories,
          documents: snapshot.navigation.documents,
          repositoryChanges: snapshot.repositoryChanges,
          repositoryError: snapshot.repositoryError,
          repositoryRevision: snapshot.repositoryRevision,
          status: snapshot.status,
        }
      : {}),
    search: searchSnapshot ? searchSnapshot.navigation.search : data.search,
  };
}

export function WorkspaceShell({
  children,
  currentPath,
  data,
  mobileContentVisible,
  query,
}: {
  children: ReactNode;
  currentPath?: string;
  data: LibraryWorkspaceView;
  mobileContentVisible: boolean;
  query?: string;
}) {
  const { settings, updateSettings } = useGlobalSettings();
  const [sidebarWidth, setSidebarWidth] = useState(
    settings.workspaceSidebarWidth,
  );
  const normalizedQuery = query?.trim() || null;
  const [snapshotState, setSnapshotState] =
    useState<WorkspaceSnapshotState | null>(null);
  const [eventStatus, setEventStatus] = useState<WorkspaceEventStatus>(
    createInitialEventStatus,
  );

  useEffect(() => {
    setSidebarWidth(settings.workspaceSidebarWidth);
  }, [settings.workspaceSidebarWidth]);

  useEffect(() => {
    let active = true;
    const events = new EventSource(createWorkspaceEventPath(normalizedQuery));
    events.onopen = () => {
      if (!active) return;
      setEventStatus((current) => ({
        ...current,
        connection: 'connected',
      }));
    };
    events.onmessage = (event) => {
      if (!active) return;
      const nextSnapshot = parseWorkspaceEventSnapshot(event.data);
      if (nextSnapshot) {
        setSnapshotState({ query: normalizedQuery, snapshot: nextSnapshot });
        setEventStatus({
          connection: 'connected',
          lastEventAt: eventTimeFormatter.format(new Date()),
        });
      }
    };
    events.onerror = () => {
      if (!active) return;
      setEventStatus((current) => ({
        ...current,
        connection: 'reconnecting',
      }));
    };
    return () => {
      active = false;
      events.close();
    };
  }, [normalizedQuery]);

  const liveLibraryData = useMemo(
    () => mergeWorkspaceLibraryData(data, snapshotState, normalizedQuery),
    [data, normalizedQuery, snapshotState],
  );
  return (
    <WorkspaceLibraryDataContext.Provider value={liveLibraryData}>
      <div
        className="grid h-dvh min-w-0 grid-cols-1 bg-background text-sm md:grid-cols-[var(--workspace-sidebar-width)_0_minmax(0,1fr)]"
        style={
          {
            '--workspace-sidebar-width': `${sidebarWidth}px`,
          } as CSSProperties
        }
      >
        <WorkspaceSidebar
          currentPath={currentPath}
          data={liveLibraryData}
          eventStatus={eventStatus}
          hiddenOnMobile={mobileContentVisible}
          query={query}
        />
        <WorkspaceSidebarResizeHandle
          onWidthChange={setSidebarWidth}
          onWidthCommit={(width) => {
            void updateSettings({ workspaceSidebarWidth: width }).catch(() =>
              setSidebarWidth(settings.workspaceSidebarWidth),
            );
          }}
          width={sidebarWidth}
        />
        <div
          className={cn(
            'h-full min-h-0 min-w-0',
            mobileContentVisible ? 'block' : 'hidden md:block',
          )}
          data-workspace-mobile-pane="secondary"
        >
          {children}
        </div>
        {mobileContentVisible ? <WorkspaceMobileEdgeSwipe /> : null}
      </div>
    </WorkspaceLibraryDataContext.Provider>
  );
}

export function useWorkspaceLibraryData(): LibraryWorkspaceView {
  const data = useContext(WorkspaceLibraryDataContext);
  if (!data) {
    throw new Error('useWorkspaceLibraryData 必须在 WorkspaceShell 内使用。');
  }
  return data;
}
