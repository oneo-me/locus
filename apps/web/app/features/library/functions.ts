import type {
  LibraryGraph,
  ReorderDocumentsInput,
  SetDocumentGraphParticipationInput,
  UpdateDocumentIconImageInput,
  UpdateDocumentMetadataInput,
} from '~/contracts/library-api';
import { apiGet, apiSend } from '~/lib/api-client';

export async function getLibraryGraph(): Promise<LibraryGraph> {
  return apiGet<LibraryGraph>('/api/library/graph');
}

export function updateDocumentMetadata(data: UpdateDocumentMetadataInput) {
  return apiSend<unknown>('/api/library/document/metadata', 'POST', data);
}

export function updateDocumentIconImage(data: UpdateDocumentIconImageInput) {
  return apiSend<unknown>('/api/library/document/icon', 'POST', data);
}

export function reorderDocuments(data: ReorderDocumentsInput) {
  return apiSend<{ ok: true }>('/api/library/reorder', 'POST', data);
}

export function setDocumentGraphParticipation(
  data: SetDocumentGraphParticipationInput,
): Promise<{ ok: true; participates: boolean }> {
  return apiSend<{ ok: true; participates: boolean }>(
    '/api/library/graph-participation',
    'POST',
    data,
  );
}
