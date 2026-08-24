import { createWebAppManifestResponse } from '~/server/manifest';
import type { Route } from './+types/manifest';

export async function loader({ request }: Route.LoaderArgs) {
  return createWebAppManifestResponse(request);
}
