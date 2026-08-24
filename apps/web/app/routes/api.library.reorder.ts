import type { ReorderDocumentsInput } from '~/contracts/library-api';
import { reorderDocuments } from '~/server/features/library/service';
import { jsonError, jsonOk, readJsonBody } from '~/server/http/json';
import type { Route } from './+types/api.library.reorder';

export async function action({ request }: Route.ActionArgs) {
  if (request.method !== 'POST') {
    return jsonError(new Error('Method Not Allowed'), 405);
  }
  try {
    const body = await readJsonBody<ReorderDocumentsInput>(request);
    return jsonOk(await reorderDocuments(body));
  } catch (error) {
    return jsonError(error);
  }
}
