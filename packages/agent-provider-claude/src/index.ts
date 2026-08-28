import type { AgentProviderExtension } from '@locus/agent/server/provider-extension';
import { discoverModels, fallbackModels } from './models.js';
import { ClaudeRuntime } from './runtime.js';

export const claudeProvider: AgentProviderExtension = {
  command: 'claude',
  createRuntime: ({
    binary,
    conversation,
    handlers,
    models,
    selectedModel,
    selectedReasoningEffort,
  }) =>
    ClaudeRuntime.start({
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
  id: 'claude',
  name: 'Claude Code',
  restartOnConfigChange: (option, value) =>
    option.category === 'thought_level' && typeof value === 'string',
};

export { ClaudeRuntime } from './runtime.js';
