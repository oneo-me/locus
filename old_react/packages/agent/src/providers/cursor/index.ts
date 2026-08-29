import { AcpRuntime } from '../../server/acp-runtime.js';
import type { AgentProvider } from '../protocol.js';
import { discoverModels, fallbackModels } from './models.js';

export const cursorProvider: AgentProvider = {
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
