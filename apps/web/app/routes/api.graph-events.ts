import { createGraphEventsResponse } from '~/server/features/graph/graph-events';
import type { Route } from './+types/api.graph-events';

export async function loader({ request }: Route.LoaderArgs) {
  return createGraphEventsResponse(request);
}
