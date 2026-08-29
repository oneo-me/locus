import type { AgentProvider } from '../protocol.js';
import { discoverModels, fallbackModels } from './models.js';
import { CodexRuntime } from './runtime.js';

export const codexProvider: AgentProvider = {
  command: 'codex',
  createRuntime: ({
    binary,
    conversation,
    handlers,
    models,
    selectedModel,
    selectedReasoningEffort,
  }) =>
    CodexRuntime.start({
      binary,
      cwd: conversation.cwd,
      handlers: handlers.native,
      models,
      selectedModel,
      selectedReasoningEffort,
      sessionId: conversation.providerSessionId,
    }),
  discoverModels,
  fallbackModels,
  id: 'codex',
  name: 'Codex CLI',
};

export { CodexRuntime } from './runtime.js';
