import type { LibraryWorkspaceView } from '@locus/document-library/contracts/library-api';
import { LibrarySettings } from '~/features/settings/library';
import { apiGet } from '~/lib/api-client';
import type { Route } from './+types/settings-library';

export async function clientLoader() {
  try {
    const library = await apiGet<LibraryWorkspaceView>('/api/library');
    return { documents: library.documents, documentsError: null };
  } catch (cause) {
    return {
      documents: [],
      documentsError: cause instanceof Error ? cause.message : String(cause),
    };
  }
}

export default function SettingsLibraryRoute({
  loaderData,
}: Route.ComponentProps) {
  return (
    <LibrarySettings
      documents={loaderData.documents}
      documentsError={loaderData.documentsError}
    />
  );
}
