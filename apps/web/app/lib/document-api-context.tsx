import type {
  OpenDocumentInput,
  ReorderDocumentsInput,
  SetDocumentArchivedInput,
  SetDocumentGraphParticipationInput,
  UpdateDocumentIconImageInput,
} from '@locus/document-library/contracts/library-api';
import type {
  GlobalSettings,
  UpdateGlobalSettingsInput,
} from '@locus/document-library/contracts/global-settings';
import { createContext, type ReactNode, useContext } from 'react';

export interface DocumentApiClient {
  openDocument: (data: OpenDocumentInput) => Promise<{ ok: true }>;
  reorderDocuments: (data: ReorderDocumentsInput) => Promise<{ ok: true }>;
  setDocumentArchived: (
    data: SetDocumentArchivedInput,
  ) => Promise<{ ok: true; archived: boolean }>;
  setDocumentGraphParticipation: (
    data: SetDocumentGraphParticipationInput,
  ) => Promise<{ ok: true; participates: boolean }>;
  updateDocumentIconImage: (
    data: UpdateDocumentIconImageInput,
  ) => Promise<unknown>;
  updateGlobalSettings: (
    data: UpdateGlobalSettingsInput,
  ) => Promise<GlobalSettings>;
}

const DocumentApiContext = createContext<DocumentApiClient | null>(null);

export function DocumentApiProvider({
  children,
  client,
}: {
  children: ReactNode;
  client: DocumentApiClient;
}) {
  return (
    <DocumentApiContext.Provider value={client}>
      {children}
    </DocumentApiContext.Provider>
  );
}

export function useDocumentApi(): DocumentApiClient {
  const client = useContext(DocumentApiContext);
  if (!client) throw new Error('DocumentApiProvider 未配置。');
  return client;
}
