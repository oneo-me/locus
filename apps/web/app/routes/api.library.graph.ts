import { getLibraryGraph } from '~/server/features/library/service';
import { jsonError, jsonOk } from '~/server/http/json';

export async function loader() {
  try {
    return jsonOk(await getLibraryGraph());
  } catch (error) {
    return jsonError(error);
  }
}
