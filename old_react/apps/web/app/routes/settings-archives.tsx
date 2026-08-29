import type { LibraryArchiveView } from '@locus/document-library/contracts/library-api';
import { ArchiveSettings } from '~/features/settings/archives';
import { apiGet } from '~/lib/api-client';
import type { Route } from './+types/settings-archives';

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

export default function SettingsArchivesRoute({
  loaderData,
}: Route.ComponentProps) {
  return (
    <ArchiveSettings
      data={loaderData.archives}
      error={loaderData.archivesError}
    />
  );
}
