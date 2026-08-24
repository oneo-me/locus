import { GraphBrowser } from '~/features/graph/browser';
import { getLibraryGraph } from '~/server/features/library/service';
import type { ShouldRevalidateFunctionArgs } from 'react-router';
import type { Route } from './+types/workspace-graph';

export async function loader(_args: Route.LoaderArgs) {
  return getLibraryGraph();
}

export function shouldRevalidate({
  currentUrl,
  defaultShouldRevalidate,
  formMethod,
  nextUrl,
}: ShouldRevalidateFunctionArgs): boolean {
  if (formMethod) return defaultShouldRevalidate;
  if (
    currentUrl.pathname === nextUrl.pathname &&
    currentUrl.search !== nextUrl.search
  ) {
    return false;
  }
  return defaultShouldRevalidate;
}

export default function GraphRoute({ loaderData }: Route.ComponentProps) {
  return <GraphBrowser data={loaderData} />;
}
