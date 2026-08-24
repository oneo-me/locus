import { getLibraryWorkspaceView } from '~/server/features/library/service';
import { jsonError, jsonOk } from '~/server/http/json';
import type { Route } from './+types/api.library';

export async function loader({ request }: Route.LoaderArgs) {
  try {
    const url = new URL(request.url);
    return jsonOk(
      await getLibraryWorkspaceView({
        query: url.searchParams.get('q') ?? undefined,
      }),
    );
  } catch (error) {
    return jsonError(error);
  }
}
