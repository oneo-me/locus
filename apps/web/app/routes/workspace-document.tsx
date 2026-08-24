import {
  pathnameToUrlPath,
  urlPathToDocumentPath,
} from '~/contracts/library-route';
import { DocumentBrowser } from '~/features/document/browser';
import { useWorkspaceLibraryData } from '~/features/workspace/shell';
import { getLibrarySelection } from '~/server/features/library/service';
import { useEffect } from 'react';
import {
  type ShouldRevalidateFunctionArgs,
  useLocation,
  useNavigation,
  useRevalidator,
} from 'react-router';
import type { Route } from './+types/workspace-document';

export function resolveDocumentLoaderPathname(pathname: string): string {
  return pathname.endsWith('/_.data')
    ? pathname.replace(/_\.data$/, '')
    : pathname.replace(/\.data$/, '');
}

export async function loader({ request }: Route.LoaderArgs) {
  const pathname = resolveDocumentLoaderPathname(new URL(request.url).pathname);
  return getLibrarySelection({
    doc: urlPathToDocumentPath(pathnameToUrlPath(pathname)),
  });
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
