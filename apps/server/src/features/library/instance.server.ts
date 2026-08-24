import { DocumentLibrary } from '@locus/document-library/server/document-library';
import { readGlobalSettings } from '~/features/settings/global-settings.server';

let libraryPromise: Promise<DocumentLibrary> | undefined;

export function getLibrary(): Promise<DocumentLibrary> {
  if (!libraryPromise) {
    const rootPath = readGlobalSettings().libraryPath;
    if (!rootPath) {
      throw new Error('请先在设置中填写文档库路径。');
    }
    libraryPromise = DocumentLibrary.open(rootPath);
  }
  return libraryPromise;
}

export async function closeLibrary(): Promise<void> {
  const library = await libraryPromise;
  libraryPromise = undefined;
  library?.close();
}
