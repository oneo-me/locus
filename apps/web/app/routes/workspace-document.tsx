import {
  pathnameToUrlPath,
  urlPathToDocumentPath,
} from '@locus/document-library/contracts/library-route';
import type { LibrarySelectionSourceView } from '@locus/document-library/contracts/library-api';
import { DocumentBrowser } from '~/features/document/browser';
import { prepareLibrarySelectionView } from '~/features/document/markdown-parser';
import { useWorkspaceLibraryData } from '~/features/workspace/shell';
import { apiGet } from '~/lib/api-client';
import { useEffect } from 'react';
import {
  type ShouldRevalidateFunctionArgs,
  useLocation,
  useNavigation,
  useRevalidator,
} from 'react-router';
import type { Route } from './+types/workspace-document';

export async function clientLoader({ request }: Route.ClientLoaderArgs) {
  const pathname = new URL(request.url).pathname;
  const source = await apiGet<LibrarySelectionSourceView>(
    '/api/library/document',
    { doc: urlPathToDocumentPath(pathnameToUrlPath(pathname)) },
  );
  return prepareLibrarySelectionView(source);
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

export default function WorkspaceDocumentRoute({
  loaderData,
}: Route.ComponentProps) {
  const workspace = useWorkspaceLibraryData();
  const location = useLocation();
  const navigation = useNavigation();
  const revalidator = useRevalidator();
  const selectionPending =
    navigation.state === 'loading' &&
    navigation.location.pathname !== location.pathname;

  useEffect(() => {
    if (
      navigation.state === 'idle' &&
      revalidator.state === 'idle' &&
      loaderData.indexedAt !== workspace.status.indexedAt
    ) {
      void revalidator.revalidate();
    }
  }, [
    loaderData.indexedAt,
    navigation.state,
    revalidator,
    workspace.status.indexedAt,
  ]);

  return (
    <DocumentBrowser
      data={{ ...workspace, ...loaderData }}
      selectionPending={selectionPending}
    />
  );
}
