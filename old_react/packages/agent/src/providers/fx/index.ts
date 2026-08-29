import { AcpRuntime } from '../../server/acp-runtime.js';
import type { AgentProvider } from '../protocol.js';
import { discoverModels, fallbackModels } from './models.js';

export const fxProvider: AgentProvider = {
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
