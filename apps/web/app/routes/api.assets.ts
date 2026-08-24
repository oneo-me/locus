import { createRouteAssetResponse } from '~/server/features/asset/route.server';
import type { Route } from './+types/api.assets';

export async function loader({ request }: Route.LoaderArgs) {
  const routePath = new URL(request.url).searchParams.get('path');
  if (!routePath) {
    return new Response('缺少或无法识别附件路径。', { status: 400 });
  }
  return (
    (await createRouteAssetResponse(request, routePath)) ??
    new Response('附件不存在。', { status: 404 })
  );
}
