import type { UpdateDocumentMetadataInput } from '~/contracts/library-api';
import { updateDocumentMetadata } from '~/server/features/library/service';
import { jsonError, jsonOk, readJsonBody } from '~/server/http/json';
import type { Route } from './+types/api.library.document.metadata';

export async function action({ request }: Route.ActionArgs) {
  if (request.method !== 'POST') {
    return jsonError(new Error('Method Not Allowed'), 405);
  }
  try {
    const body = await readJsonBody<UpdateDocumentMetadataInput>(request);
    return jsonOk(await updateDocumentMetadata(body));
  } catch (error) {
    return jsonError(error);
  }
}
