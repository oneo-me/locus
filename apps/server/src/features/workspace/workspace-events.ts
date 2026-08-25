import type { DocumentLibrary } from '@locus/document-library/server/document-library';
import { getLibrary } from '~/features/library/instance.server';
import { readLibraryNavigation } from '~/features/library/navigation';
import {
  type LibraryRepositoryView,
  readLibraryRepositoryView,
  repositoryViewChanged,
} from '~/features/library/repository-view.server';

const encoder = new TextEncoder();
const HEARTBEAT_INTERVAL_MS = 5_000;
const SNAPSHOT_DEBOUNCE_MS = 150;

export async function createWorkspaceEventsResponse(
  request: Request,
): Promise<Response> {
  const library: DocumentLibrary = await getLibrary();
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
        repositoryRevision = nextRepositoryView.revision;
        enqueue(
          `data: ${JSON.stringify({
            status: library.status(),
            repositoryChanges: nextRepositoryView.changes,
            repositoryError: nextRepositoryView.error,
            repositoryRevision: nextRepositoryView.revision,
            navigation: readLibraryNavigation(library),
          })}\n\n`,
        );
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
        snapshotTimer = setTimeout(flushSnapshot, SNAPSHOT_DEBOUNCE_MS);
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
      }, HEARTBEAT_INTERVAL_MS);

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
