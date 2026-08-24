import type { LibraryWorkspaceView } from '@locus/document-library/contracts/library-api';
import type { GlobalSettings } from '@locus/document-library/contracts/global-settings';
import { pathnameToUrlPath } from '@locus/document-library/contracts/library-route';
import { ThemeProvider } from '@locus/ui/theme-provider';
import { parseLibrarySearch } from '~/features/library/search';
import { GlobalSettingsProvider } from '~/features/settings/global-settings-provider';
import { LibrarySetup } from '~/features/settings/library-setup';
import { workspaceShowsMobileContent } from '~/features/workspace/mobile-navigation';
import { WorkspaceErrorState } from '~/features/workspace/route-state';
import { WorkspaceShell } from '~/features/workspace/shell';
import { apiGet } from '~/lib/api-client';
import { documentApi } from '~/lib/document-api';
import { DocumentApiProvider } from '~/lib/document-api-context';
import { useEffect } from 'react';
import {
  isRouteErrorResponse,
  Outlet,
  redirect,
  type ShouldRevalidateFunctionArgs,
  useLocation,
  useRouteError,
  useSearchParams,
} from 'react-router';
import type { Route } from './+types/workspace';

export async function clientLoader({ request }: Route.ClientLoaderArgs) {
  const url = new URL(request.url);
  if (url.pathname === '/document') throw redirect('/');

  const search = parseLibrarySearch({
    q: url.searchParams.get('q') ?? undefined,
  });
  const settings = await apiGet<GlobalSettings>('/api/settings');
  if (!settings.libraryPath) {
    return { library: null, libraryError: null, settings };
  }

  try {
    const library = await apiGet<LibraryWorkspaceView>('/api/library', {
      q: search.q,
    });
    return { library, libraryError: null, settings };
  } catch (cause) {
    return {
      library: null,
      libraryError: cause instanceof Error ? cause.message : String(cause),
      settings,
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
    currentUrl.search === nextUrl.search
  ) {
    return defaultShouldRevalidate;
  }
  return currentUrl.searchParams.get('q') !== nextUrl.searchParams.get('q');
}

export default function WorkspaceRoot({ loaderData }: Route.ComponentProps) {
  const { library, libraryError, settings } = loaderData;
  const location = useLocation();
  const { pathname } = location;
  const [searchParams] = useSearchParams();
  const query = parseLibrarySearch({
    q: searchParams.get('q') ?? undefined,
  }).q;

  useEffect(() => {
    document.querySelector<HTMLElement>('#main-content')?.scrollTo({
      left: 0,
      top: 0,
    });
  }, []);

  const currentPath =
    pathname !== '/' ? pathnameToUrlPath(pathname) : undefined;

  return (
    <DocumentApiProvider client={documentApi}>
      <ThemeProvider
        defaultTheme={settings.theme}
        onThemeChange={(theme) => documentApi.updateGlobalSettings({ theme })}
      >
        <GlobalSettingsProvider initialSettings={settings}>
          {library ? (
            <WorkspaceShell
              currentPath={currentPath}
              data={library}
              mobileContentVisible={
                workspaceShowsMobileContent(location.state) && pathname !== '/'
              }
              query={query}
            >
              <Outlet />
            </WorkspaceShell>
          ) : (
            <LibrarySetup error={libraryError ?? undefined} />
          )}
        </GlobalSettingsProvider>
      </ThemeProvider>
    </DocumentApiProvider>
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
