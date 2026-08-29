import { AgentService } from '@locus/agent/server/service';
import { ensureDataDirectory } from '~/data-directory.server';
import { agentProviders } from './providers.server';

let agentService: AgentService | undefined;

export function getAgentService(): AgentService {
  agentService ??= new AgentService(ensureDataDirectory(), agentProviders);
  return agentService;
}

export function closeAgentService(): void {
  agentService?.close();
  agentService = undefined;
}
