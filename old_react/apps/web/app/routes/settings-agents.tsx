import type { AgentProviderDetection } from '@locus/agent/contracts/provider';
import { AgentSettings } from '~/features/settings/agents';
import { apiGet } from '~/lib/api-client';
import type { Route } from './+types/settings-agents';

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
        providers: [],
      },
      error: cause instanceof Error ? cause.message : String(cause),
    };
  }
}

export default function SettingsAgentsRoute({
  loaderData,
}: Route.ComponentProps) {
  return (
    <AgentSettings
      initialDetection={loaderData.detection}
      initialError={loaderData.error}
    />
  );
}
