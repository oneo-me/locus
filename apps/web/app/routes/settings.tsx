import { SettingsBrowser } from '~/features/settings/browser';
import { useLocation } from 'react-router';
import type { LibraryArchiveView } from '@locus/document-library/contracts/library-api';
import { apiGet } from '~/lib/api-client';
import type { Route } from './+types/settings';

export async function clientLoader() {
  try {
    return {
      archives: await apiGet<LibraryArchiveView>('/api/library/archives'),
      archivesError: null,
    };
  } catch (cause) {
    return {
      archives: { documents: [] },
      archivesError: cause instanceof Error ? cause.message : String(cause),
    };
  }
}

export default function SettingsRoute({ loaderData }: Route.ComponentProps) {
  const location = useLocation();
  const libraryError =
    location.state !== null &&
    typeof location.state === 'object' &&
    'libraryError' in location.state &&
    typeof location.state.libraryError === 'string'
      ? location.state.libraryError
      : undefined;
  return (
    <SettingsBrowser
      archives={loaderData.archives}
      archivesError={loaderData.archivesError}
      libraryError={libraryError}
    />
  );
}
