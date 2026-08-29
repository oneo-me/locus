import { AcpRuntime } from '../../server/acp-runtime.js';
import type { AgentProvider } from '../protocol.js';
import { discoverModels, fallbackModels } from './models.js';

export const kimiProvider: AgentProvider = {
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
