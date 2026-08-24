import type { SetDocumentGraphParticipationInput } from '~/contracts/library-api';
import { setDocumentGraphParticipation } from '~/server/features/library/service';
import { jsonError, jsonOk, readJsonBody } from '~/server/http/json';
import type { Route } from './+types/api.library.graph-participation';

export async function action({ request }: Route.ActionArgs) {
  if (request.method !== 'POST') {
    return jsonError(new Error('Method Not Allowed'), 405);
  }
  try {
    const body =
      await readJsonBody<SetDocumentGraphParticipationInput>(request);
    return jsonOk(await setDocumentGraphParticipation(body));
  } catch (error) {
    return jsonError(error);
  }
}
