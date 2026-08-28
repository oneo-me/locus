import type { AgentProviderExtension } from '@locus/agent/server/provider-extension';
import { discoverModels, fallbackModels } from './models.js';
import { OpenCodeRuntime } from './runtime.js';

export const opencodeProvider: AgentProviderExtension = {
  command: 'opencode',
  createRuntime: ({ binary, conversation, handlers, models, selectedModel }) =>
    OpenCodeRuntime.start({
      binary,
      cwd: conversation.cwd,
      handlers: handlers.native,
      models,
      selectedModel,
      sessionId: conversation.providerSessionId,
    }),
  discoverModels,
  fallbackModels,
  id: 'opencode',
  name: 'OpenCode',
  normalizeTitle: (title) =>
    title?.trim().startsWith('New session - ')
      ? undefined
      : title?.trim() || null,
};

export { OpenCodeRuntime } from './runtime.js';
