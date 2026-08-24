import type { DocumentLibrary } from '@locus/document-library/server/document-library';
import { getLibrary } from '~/features/library/instance.server';
import { readLibraryNavigation } from '~/features/library/navigation';
import {
  type LibraryRepositoryView,
  readLibraryRepositoryView,
  repositoryViewChanged,
} from '~/features/library/repository-view.server';
import {
  excludedDocumentGraphPaths,
  type LibraryState,
} from '@locus/document-library/contracts/library-state';
import { readLibraryState } from '~/features/library/state.server';

const encoder = new TextEncoder();
const HEARTBEAT_INTERVAL_MS = 5_000;
const SNAPSHOT_DEBOUNCE_MS = 150;

interface GraphEventsResponseOptions {
  heartbeatIntervalMs?: number;
  library?: DocumentLibrary;
  readState?: () => LibraryState;
  snapshotDebounceMs?: number;
}

export async function createGraphEventsResponse(
  request: Request,
  options: GraphEventsResponseOptions = {},
): Promise<Response> {
  const library = options.library ?? (await getLibrary());
  const heartbeatIntervalMs =
    options.heartbeatIntervalMs ?? HEARTBEAT_INTERVAL_MS;
  const readState = options.readState ?? readLibraryState;
  const snapshotDebounceMs = options.snapshotDebounceMs ?? SNAPSHOT_DEBOUNCE_MS;
  const params = new URL(request.url).searchParams;
  const query = params.get('q')?.trim();
  const includeGraph = params.get('graph') === '1';
  let cleanup = () => {};
  let flushPendingSnapshot = () => {};

  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      let closed = false;
      let unsubscribe = () => {};
      let repositoryRevision: string | null = null;
      let snapshotPending = false;
      let pendingRepositoryView: LibraryRepositoryView | undefined;
      let snapshotTimer: ReturnType<typeof setTimeout> | null = null;

      const close = () => {
        if (closed) return;
        closed = true;
        if (snapshotTimer) clearTimeout(snapshotTimer);
        snapshotTimer = null;
        clearInterval(heartbeat);
        unsubscribe();
        request.signal.removeEventListener('abort', close);
        flushPendingSnapshot = () => {};
        try {
          controller.close();
        } catch {
          // The client may already have cancelled the stream.
        }
      };

      const enqueue = (frame: string): boolean => {
        if (closed) return false;
        try {
          controller.enqueue(encoder.encode(frame));
          return true;
        } catch {
          close();
          return false;
        }
      };

      const sendSnapshot = (repositoryView?: LibraryRepositoryView) => {
        const nextRepositoryView =
          repositoryView ?? readLibraryRepositoryView(library);
        const state = readState();
        const navigation = readLibraryNavigation(library, query);
        repositoryRevision = nextRepositoryView.revision;
        const snapshot = {
          status: library.status(),
          ...(includeGraph
            ? {
                graph: library.getDocumentGraph(
                  excludedDocumentGraphPaths(
                    state,
                    library.listAllDocuments().map((document) => document.path),
                  ),
                ),
              }
            : {}),
          repositoryChanges: nextRepositoryView.changes,
          repositoryError: nextRepositoryView.error,
          repositoryRevision: nextRepositoryView.revision,
          navigation,
        };
        enqueue(`data: ${JSON.stringify(snapshot)}\n\n`);
      };

      const flushSnapshot = () => {
        snapshotTimer = null;
        if (
          closed ||
          !snapshotPending ||
          (controller.desiredSize !== null && controller.desiredSize <= 0)
        ) {
          return;
        }
        const repositoryView = pendingRepositoryView;
        snapshotPending = false;
        pendingRepositoryView = undefined;
        try {
          sendSnapshot(repositoryView);
        } catch {
          close();
        }
      };

      const scheduleSnapshot = (
        repositoryView?: LibraryRepositoryView,
        immediate = false,
      ) => {
        if (closed) return;
        snapshotPending = true;
        pendingRepositoryView = repositoryView;
        if (snapshotTimer) clearTimeout(snapshotTimer);
        snapshotTimer = null;
        if (immediate) {
          flushSnapshot();
          return;
        }
        snapshotTimer = setTimeout(flushSnapshot, snapshotDebounceMs);
      };

      const heartbeat = setInterval(() => {
        if (
          closed ||
          (controller.desiredSize !== null && controller.desiredSize <= 0)
        ) {
          return;
        }
        const nextRepositoryView = readLibraryRepositoryView(library);
        if (
          repositoryViewChanged(repositoryRevision, nextRepositoryView.revision)
        ) {
          scheduleSnapshot(nextRepositoryView);
        } else {
          enqueue(': keep-alive\n\n');
        }
      }, heartbeatIntervalMs);

      unsubscribe = library.subscribe(() => scheduleSnapshot());
      request.signal.addEventListener('abort', close, { once: true });
      cleanup = close;
      flushPendingSnapshot = () => {
        if (snapshotPending && !snapshotTimer) flushSnapshot();
      };
      scheduleSnapshot(undefined, true);
    },
    pull() {
      flushPendingSnapshot();
    },
    cancel() {
      cleanup();
    },
  });

  return new Response(stream, {
    headers: {
      'Cache-Control': 'no-cache, no-transform',
      Connection: 'keep-alive',
      'Content-Type': 'text/event-stream; charset=utf-8',
      'X-Accel-Buffering': 'no',
    },
  });
}
