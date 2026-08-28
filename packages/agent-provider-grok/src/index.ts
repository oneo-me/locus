import { AcpRuntime } from '@locus/agent/server/acp-runtime';
import type { AgentProviderExtension } from '@locus/agent/server/provider-extension';
import { discoverModels, fallbackModels } from './models.js';
import { readGrokSessionTitle } from './title.js';

export const grokProvider: AgentProviderExtension = {
  command: 'grok',
  createRuntime: ({ binary, conversation, handlers }) =>
    AcpRuntime.start({
      args: ['agent', 'stdio'],
      binary,
      cwd: conversation.cwd,
      env: { GROK_OAUTH2_REFERRER: 'locus' },
      handlers,
      sessionId: conversation.providerSessionId,
    }),
  discoverModels,
  fallbackModels,
  id: 'grok',
  name: 'Grok Build',
  normalizeConfigOptions: (options) =>
    options.map((option) =>
      option.id === 'mode' ? { ...option, category: 'thought_level' } : option,
    ),
  readSessionTitle: readGrokSessionTitle,
};
