import { DocumentLibrary } from '@locus/document-library/server/document-library';
import { readGlobalSettings } from '~/features/settings/global-settings.server';

let libraryPromise: Promise<DocumentLibrary> | undefined;
let shutdownRegistered = false;

function registerShutdown(): void {
  if (shutdownRegistered) return;
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

export function getLibrary(): Promise<DocumentLibrary> {
  if (!libraryPromise) {
    registerShutdown();
    const rootPath = readGlobalSettings().libraryPath;
    if (!rootPath) {
      throw new Error('请先在设置中填写文档库路径。');
    }
    libraryPromise = DocumentLibrary.open(rootPath, {
      watch: true,
    });
  }
  return libraryPromise;
}

export async function closeLibrary(): Promise<void> {
  const library = await libraryPromise;
  libraryPromise = undefined;
  library?.close();
}
