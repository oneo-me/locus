import {
  AGENT_PROVIDERS,
  type AgentProviderDetection,
} from '@locus/agent/contracts/provider';
import { ProviderSettings } from '~/features/settings/providers';
import { apiGet } from '~/lib/api-client';
import type { Route } from './+types/settings-providers';

export async function clientLoader() {
  try {
    return {
      detection: await apiGet<AgentProviderDetection>('/api/agent-providers'),
      error: null,
    };
  } catch (cause) {
    return {
      detection: {
        checkedAt: null,
        providers: AGENT_PROVIDERS.map((provider) => ({
          ...provider,
          installed: false,
          path: null,
        })),
      },
      error: cause instanceof Error ? cause.message : String(cause),
    };
  }
}

export default function SettingsProvidersRoute({
  loaderData,
}: Route.ComponentProps) {
  return (
    <ProviderSettings
      initialDetection={loaderData.detection}
      initialError={loaderData.error}
    />
  );
}
