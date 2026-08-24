import type { LibraryGraph } from '@locus/document-library/contracts/library-api';
import { GraphBrowser } from '~/features/graph/browser';
import { apiGet } from '~/lib/api-client';
import type { ShouldRevalidateFunctionArgs } from 'react-router';
import type { Route } from './+types/workspace-graph';

export async function clientLoader(_args: Route.ClientLoaderArgs) {
  return apiGet<LibraryGraph>('/api/library/graph');
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
