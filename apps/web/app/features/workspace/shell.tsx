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
import {
  parseWorkspaceEventSnapshot,
  type WorkspaceEventSnapshot,
  WORKSPACE_EVENTS_PATH,
} from './events';
import { useGlobalSettings } from '../settings/global-settings-provider';
import { WorkspaceSidebar } from './sidebar';
import { WorkspaceSidebarResizeHandle } from './sidebar-resize-handle';
import { cn } from '@locus/ui/utils';
import { LibraryTerminalPanel, useTerminalPanel } from '../terminal/panel';

export interface WorkspaceSnapshotState {
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
    JSON.stringify(data.repositoryChanges) ===
      JSON.stringify(snapshot.repositoryChanges) &&
    data.repositoryError === snapshot.repositoryError &&
    data.repositoryRevision === snapshot.repositoryRevision
  );
}

const WorkspaceLibraryDataContext = createContext<LibraryWorkspaceView | null>(
  null,
);

export function mergeWorkspaceLibraryData(
  data: LibraryWorkspaceView,
  snapshotState: WorkspaceSnapshotState | null,
): LibraryWorkspaceView {
  const snapshot = snapshotState?.snapshot;
  if (snapshot && liveSnapshotMatchesLoaderData(data, snapshot)) {
    return data;
  }
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
  };
}

export function WorkspaceShell({
  children,
  currentPath,
  data,
}: {
  children: ReactNode;
  currentPath?: string;
  data: LibraryWorkspaceView;
}) {
  const { settings, updateSettings } = useGlobalSettings();
  const { open: terminalOpen } = useTerminalPanel();
  const [sidebarWidthDraft, setSidebarWidthDraft] = useState<number | null>(
    null,
  );
  const sidebarWidth = sidebarWidthDraft ?? settings.workspaceSidebarWidth;
  const [snapshotState, setSnapshotState] =
    useState<WorkspaceSnapshotState | null>(null);

  useEffect(() => {
    let active = true;
    const events = new EventSource(WORKSPACE_EVENTS_PATH);
    events.onmessage = (event) => {
      if (!active) return;
      const nextSnapshot = parseWorkspaceEventSnapshot(event.data);
      if (nextSnapshot) {
        setSnapshotState({ snapshot: nextSnapshot });
      }
    };
    return () => {
      active = false;
      events.close();
    };
  }, []);

  const liveLibraryData = useMemo(
    () => mergeWorkspaceLibraryData(data, snapshotState),
    [data, snapshotState],
  );
  return (
    <WorkspaceLibraryDataContext.Provider value={liveLibraryData}>
      <div
        className={cn(
          'grid h-full min-w-0 bg-background text-sm',
          terminalOpen
            ? 'grid-cols-[var(--workspace-sidebar-width)_0_minmax(0,1fr)_minmax(20rem,40%)]'
            : 'grid-cols-[var(--workspace-sidebar-width)_0_minmax(0,1fr)]',
        )}
        style={
          {
            '--workspace-sidebar-width': `${sidebarWidth}px`,
          } as CSSProperties
        }
      >
        <WorkspaceSidebar currentPath={currentPath} data={liveLibraryData} />
        <WorkspaceSidebarResizeHandle
          onWidthChange={setSidebarWidthDraft}
          onWidthCommit={(width) => {
            void updateSettings({ workspaceSidebarWidth: width })
              .catch(() => {})
              .finally(() =>
                setSidebarWidthDraft((current) =>
                  current === width ? null : current,
                ),
              );
          }}
          width={sidebarWidth}
        />
        <div className="h-full min-h-0 min-w-0">{children}</div>
        {terminalOpen ? <LibraryTerminalPanel /> : null}
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
