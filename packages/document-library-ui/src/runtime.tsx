import {
  createContext,
  type ComponentProps,
  type ComponentType,
  type ReactNode,
  useContext,
} from 'react';
import type { MarkdownUiExtension } from '@locus/markdown-ui';

export interface DocumentLinkProps extends Omit<ComponentProps<'a'>, 'href'> {
  href: string;
}

export interface DocumentAssetUrlOptions {
  thumbnail?: boolean;
  version?: string | null;
}

export interface DocumentMarkdownExtensionContext {
  assets: string[];
  documentPath: string;
}

export interface DocumentLibraryUiRuntime {
  Link: ComponentType<DocumentLinkProps>;
  copyReference: (value: string) => Promise<void>;
  createMarkdownExtensions?: (
    context: DocumentMarkdownExtensionContext,
  ) => readonly MarkdownUiExtension[];
  getAssetUrl: (
    documentPath: string,
    assetName: string,
    options?: DocumentAssetUrlOptions,
  ) => string;
  getDocumentHref: (articlePath: string) => string;
  getRemoteFaviconUrl?: (url: URL) => string | null;
  openDocument: (
    documentPath: string,
    target: 'directory' | 'vscode',
  ) => Promise<void>;
  setDocumentArchived: (documentPath: string) => Promise<void>;
  setDocumentGraphParticipation: (
    documentPath: string,
    participates: boolean,
  ) => Promise<boolean>;
  updateDocumentIconImage: (
    documentPath: string,
    dataBase64: string,
  ) => Promise<void>;
}

const DocumentLibraryUiContext = createContext<DocumentLibraryUiRuntime | null>(
  null,
);

export function DocumentLibraryUiProvider({
  children,
  runtime,
}: {
  children: ReactNode;
  runtime: DocumentLibraryUiRuntime;
}) {
  return (
    <DocumentLibraryUiContext.Provider value={runtime}>
      {children}
    </DocumentLibraryUiContext.Provider>
  );
}

export function useDocumentLibraryUi(): DocumentLibraryUiRuntime {
  const runtime = useContext(DocumentLibraryUiContext);
  if (!runtime) throw new Error('DocumentLibraryUiProvider 未配置。');
  return runtime;
}
