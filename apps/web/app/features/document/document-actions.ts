import { libraryReferenceScheme } from '@locus/document-library/contracts/library-reference';

export async function copyDocumentText(
  value: string,
  clipboard: Pick<Clipboard, 'writeText'> | undefined = globalThis.navigator
    ?.clipboard,
): Promise<boolean> {
  if (!clipboard) return false;

  try {
    await clipboard.writeText(value);
    return true;
  } catch {
    return false;
  }
}

export function formatDocumentReference(path: string): string {
  return `${libraryReferenceScheme}${path}`;
}
