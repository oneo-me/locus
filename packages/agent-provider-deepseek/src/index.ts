import type { AgentProviderExtension } from '@locus/agent/server/provider-extension';
import { discoverModels, fallbackModels } from './models.js';
export const deepseekProvider: AgentProviderExtension = {
  command: 'dsh',
  discoverModels,
  fallbackModels,
  id: 'deepseek',
  name: 'DeepSeek Harness',
};
