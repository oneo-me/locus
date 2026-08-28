import { AcpRuntime } from '@locus/agent/server/acp-runtime';
import type { AgentProviderExtension } from '@locus/agent/server/provider-extension';
import { discoverModels, fallbackModels } from './models.js';

export const kimiProvider: AgentProviderExtension = {
  command: 'kimi',
  createRuntime: ({ binary, conversation, handlers }) =>
    AcpRuntime.start({
      args: ['acp'],
      binary,
      cwd: conversation.cwd,
      handlers,
      sessionId: conversation.providerSessionId,
    }),
  discoverModels,
  fallbackModels,
  id: 'kimi',
  name: 'Kimi Code',
};
