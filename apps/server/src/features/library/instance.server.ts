import {
  currentEnvironment,
  ensureLibraryCachePath,
} from '~/features/settings/global-paths.server';
import { DocumentLibrary } from '@locus/document-library/server/document-library';
import { resolveLibraryPath } from './path.server';

let libraryPromise: Promise<DocumentLibrary> | undefined;
let shutdownRegistered = false;

function registerProductionShutdown(): void {
  if (shutdownRegistered || process.env.NODE_ENV !== 'production') return;
  shutdownRegistered = true;
  const close = () => {
    void closeLibrary().catch((error: unknown) => {
      console.error(
        `关闭文档库失败：${error instanceof Error ? error.message : String(error)}`,
      );
    });
  };
  process.once('SIGINT', close);
  process.once('SIGTERM', close);
}

function watchUsesPolling(): boolean | undefined {
  const value = process.env.LOCUS_WATCH_USE_POLLING;
  if (value === 'true') return true;
  if (value === 'false') return false;
  return undefined;
}

function watchIntervalMs(): number | undefined {
  const value = process.env.LOCUS_WATCH_INTERVAL_MS;
  if (!value) return undefined;
  const interval = Number.parseInt(value, 10);
  return Number.isInteger(interval) && interval >= 100 ? interval : undefined;
}

export function getLibrary(): Promise<DocumentLibrary> {
  if (!libraryPromise) {
    registerProductionShutdown();
    const rootPath = resolveLibraryPath();
    libraryPromise = DocumentLibrary.open(rootPath, {
      cachePath: ensureLibraryCachePath(currentEnvironment()),
      watch: true,
      watchIntervalMs: watchIntervalMs(),
      watchUsePolling: watchUsesPolling(),
    });
  }
  return libraryPromise;
}

export async function closeLibrary(): Promise<void> {
  const library = await libraryPromise;
  libraryPromise = undefined;
  library?.close();
}
