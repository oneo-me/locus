import { AcpRuntime } from '@locus/agent/server/acp-runtime';
import type { AgentProviderExtension } from '@locus/agent/server/provider-extension';
import { discoverModels, fallbackModels } from './models.js';

export const cursorProvider: AgentProviderExtension = {
  command: 'cursor-agent',
  createRuntime: ({ binary, conversation, handlers }) =>
    AcpRuntime.start({
      args: ['acp'],
      binary,
      cwd: conversation.cwd,
      handlers,
      parameterizedModelPicker: true,
      sessionId: conversation.providerSessionId,
      setModelWithConfig: true,
    }),
  discoverModels,
  fallbackModels,
  id: 'cursor',
  name: 'Cursor CLI',
};
