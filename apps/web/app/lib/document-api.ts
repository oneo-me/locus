import type { DocumentApiClient } from './document-api-context';
import { apiSend } from './api-client';

export const documentApi: DocumentApiClient = {
  openDocument: (data) => apiSend('/api/library/document/open', 'POST', data),
  openLibraryTerminal: () => apiSend('/api/library/terminal/open', 'POST'),
  reorderDocuments: (data) => apiSend('/api/library/reorder', 'POST', data),
  setDocumentArchived: (data) => apiSend('/api/library/archive', 'POST', data),
  setDocumentGraphParticipation: (data) =>
    apiSend('/api/library/graph-participation', 'POST', data),
  updateDocumentIconImage: (data) =>
    apiSend('/api/library/document/icon', 'POST', data),
  updateGlobalSettings: (data) => apiSend('/api/settings', 'PATCH', data),
};
