import type { AgentProvider } from '../protocol.js';
import { discoverModels, fallbackModels } from './models.js';
import { PiRuntime } from './runtime.js';

export const piProvider: AgentProvider = {
  command: 'pi',
  createRuntime: ({
    binary,
    conversation,
    handlers,
    models,
    selectedModel,
    selectedReasoningEffort,
  }) =>
    PiRuntime.start({
      binary,
      cwd: conversation.cwd,
      handlers: handlers.native,
      models,
      selectedModel,
      selectedReasoningEffort,
      sessionPath: conversation.providerSessionPath,
    }),
  discoverModels,
  fallbackModels,
  id: 'pi',
  name: 'Pi',
};

export { PiRuntime } from './runtime.js';
