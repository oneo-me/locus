import type { DocumentApiClient } from './document-api-context';
import { apiSend } from './api-client';

export const documentApi: DocumentApiClient = {
  reorderDocuments: (data) => apiSend('/api/library/reorder', 'POST', data),
  updateDocumentIconImage: (data) =>
    apiSend('/api/library/document/icon', 'POST', data),
  updateGlobalSettings: (data) => apiSend('/api/settings', 'PATCH', data),
};
