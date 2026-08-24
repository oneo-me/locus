import type { UpdateDocumentIconImageInput } from '~/contracts/library-api';
import { updateDocumentIconImage } from '~/server/features/library/service';
import { jsonError, jsonOk, readJsonBody } from '~/server/http/json';
import type { Route } from './+types/api.library.document.icon';

export async function action({ request }: Route.ActionArgs) {
  if (request.method !== 'POST') {
    return jsonError(new Error('Method Not Allowed'), 405);
  }
  try {
    const body = await readJsonBody<UpdateDocumentIconImageInput>(request);
    return jsonOk(await updateDocumentIconImage(body));
  } catch (error) {
    return jsonError(error);
  }
}
