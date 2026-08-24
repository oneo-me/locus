import { pathnameToUrlPath } from '~/contracts/library-route';
import { ThemeProvider } from '~/components/theme-provider';
import { parseLibrarySearch } from '~/features/library/search';
import { GlobalSettingsProvider } from '~/features/settings/global-settings-provider';
import { workspaceShowsMobileContent } from '~/features/workspace/mobile-navigation';
import { WorkspaceErrorState } from '~/features/workspace/route-state';
import { WorkspaceShell, type WorkspaceView } from '~/features/workspace/shell';
import { getLibraryWorkspaceView } from '~/server/features/library/service';
import { getGlobalSettings } from '~/server/features/settings/service';
import { dynamicContentCacheControl } from '~/server/http/cache';
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

export async function loader({ request }: Route.LoaderArgs) {
  const url = new URL(request.url);
  if (url.pathname === '/document') throw redirect('/');

  const search = parseLibrarySearch({
    q: url.searchParams.get('q') ?? undefined,
  });
  const library = await getLibraryWorkspaceView({
    query: search.q,
  });
  const settings = getGlobalSettings();
  return { library, settings };
}

export function headers() {
  return { 'Cache-Control': dynamicContentCacheControl };
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
  const { library, settings } = loaderData;
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

  const activeView: WorkspaceView =
    pathname === '/graph' ? 'graph' : 'documents';
  const currentPath =
    activeView === 'documents' && pathname !== '/'
      ? pathnameToUrlPath(pathname)
      : undefined;

  return (
    <ThemeProvider>
      <GlobalSettingsProvider initialSettings={settings}>
        <WorkspaceShell
          activeView={activeView}
          currentPath={currentPath}
          data={library}
          mobileContentVisible={
            workspaceShowsMobileContent(activeView, location.state) &&
            pathname !== '/'
          }
          query={query}
        >
          <Outlet />
        </WorkspaceShell>
      </GlobalSettingsProvider>
    </ThemeProvider>
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
