import type { DocumentApiClient } from './document-api-context';
import { apiSend } from './api-client';

export const documentApi: DocumentApiClient = {
  openLocalDirectory: (data) =>
    apiSend('/api/system/directory/open', 'POST', data),
  openDocument: (data) => apiSend('/api/library/document/open', 'POST', data),
  reorderDocuments: (data) => apiSend('/api/library/reorder', 'POST', data),
  setDocumentArchived: (data) => apiSend('/api/library/archive', 'POST', data),
  setDocumentGraphParticipation: (data) =>
    apiSend('/api/library/graph-participation', 'POST', data),
  updateDocumentIconImage: (data) =>
    apiSend('/api/library/document/icon', 'POST', data),
  updateGlobalSettings: (data) => apiSend('/api/settings', 'PATCH', data),
};
