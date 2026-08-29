import type { GlobalSettings } from '@locus/document-library/contracts/global-settings';
import { ThemeProvider } from '@locus/ui/theme-provider';
import { GlobalSettingsProvider } from '~/features/settings/global-settings-provider';
import { AppShell } from '~/features/workspace/app-shell';
import { apiGet } from '~/lib/api-client';
import { documentApi } from '~/lib/document-api';
import { DocumentApiProvider } from '~/lib/document-api-context';
import { Outlet } from 'react-router';
import type { Route } from './+types/app';

export async function clientLoader() {
  return apiGet<GlobalSettings>('/api/settings');
}

export default function AppRoute({
  loaderData: settings,
}: Route.ComponentProps) {
  return (
    <DocumentApiProvider client={documentApi}>
      <ThemeProvider
        defaultTheme={settings.theme}
        onThemeChange={(theme) => documentApi.updateGlobalSettings({ theme })}
      >
        <GlobalSettingsProvider initialSettings={settings}>
          <AppShell>
            <Outlet />
          </AppShell>
        </GlobalSettingsProvider>
      </ThemeProvider>
    </DocumentApiProvider>
  );
}
