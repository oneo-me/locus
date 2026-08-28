import { AcpRuntime } from '@locus/agent/server/acp-runtime';
import type { AgentProviderExtension } from '@locus/agent/server/provider-extension';
import { discoverModels, fallbackModels } from './models.js';

export const fxProvider: AgentProviderExtension = {
  command: 'fx',
  createRuntime: ({ binary, conversation, handlers }) =>
    AcpRuntime.start({
      args: ['acp'],
      binary,
      cwd: conversation.cwd,
      handlers,
      sessionId: conversation.providerSessionId,
      setModelWithConfig: true,
    }),
  discoverModels,
  fallbackModels,
  id: 'fx',
  name: 'Fx',
};
