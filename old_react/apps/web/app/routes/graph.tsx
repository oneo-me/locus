import type { LibraryGraph } from '@locus/document-library/contracts/library-api';
import type { GlobalSettings } from '@locus/document-library/contracts/global-settings';
import { GraphBrowser } from '~/features/graph/browser';
import { LibrarySetup } from '~/features/settings/library-setup';
import { apiGet } from '~/lib/api-client';
import type { ShouldRevalidateFunctionArgs } from 'react-router';
import type { Route } from './+types/graph';

export async function clientLoader() {
  const settings = await apiGet<GlobalSettings>('/api/settings');
  if (!settings.libraryPath) {
    return { graph: null, graphError: null };
  }

  try {
    const graph = await apiGet<LibraryGraph>('/api/library/graph');
    return { graph, graphError: null };
  } catch (cause) {
    return {
      graph: null,
      graphError: cause instanceof Error ? cause.message : String(cause),
    };
  }
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
  return loaderData.graph ? (
    <GraphBrowser data={loaderData.graph} />
  ) : (
    <LibrarySetup error={loaderData.graphError ?? undefined} />
  );
}

export function meta() {
  return [{ title: '知识图谱 — Locus' }];
}
