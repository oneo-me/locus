import type { LibraryWorkspaceView } from '@locus/document-library/contracts/library-api';
import type { GlobalSettings } from '@locus/document-library/contracts/global-settings';
import { pathnameToUrlPath } from '@locus/document-library/contracts/library-route';
import { ThemeProvider } from '@locus/ui/theme-provider';
import { LibrarySetup } from '~/features/settings/library-setup';
import { WorkspaceErrorState } from '~/features/workspace/route-state';
import { WorkspaceShell } from '~/features/workspace/shell';
import { apiGet } from '~/lib/api-client';
import { useEffect } from 'react';
import {
  isRouteErrorResponse,
  Outlet,
  useLocation,
  useRouteError,
} from 'react-router';
import type { Route } from './+types/workspace';

export async function clientLoader() {
  const settings = await apiGet<GlobalSettings>('/api/settings');
  if (!settings.libraryPath) {
    return { library: null, libraryError: null };
  }

  try {
    const library = await apiGet<LibraryWorkspaceView>('/api/library');
    return { library, libraryError: null };
  } catch (cause) {
    return {
      library: null,
      libraryError: cause instanceof Error ? cause.message : String(cause),
    };
  }
}

export default function WorkspaceRoot({ loaderData }: Route.ComponentProps) {
  const { library, libraryError } = loaderData;
  const location = useLocation();
  const { pathname } = location;

  useEffect(() => {
    document.querySelector<HTMLElement>('#main-content')?.scrollTo({
      left: 0,
      top: 0,
    });
  }, []);

  const currentPath =
    pathname !== '/notes' ? pathnameToUrlPath(pathname) : undefined;

  return library ? (
    <WorkspaceShell currentPath={currentPath} data={library}>
      <Outlet />
    </WorkspaceShell>
  ) : (
    <LibrarySetup error={libraryError ?? undefined} />
  );
}

export function ErrorBoundary() {
  const cause = useRouteError();
  const error =
    cause instanceof Error
      ? cause
      : new Error(
          isRouteErrorResponse(cause)
            ? cause.statusText || `请求失败（${cause.status}）。`
            : String(cause),
        );
  return (
    <ThemeProvider>
      <WorkspaceErrorState error={error} />
    </ThemeProvider>
  );
}
