import type { AgentProviderExtension } from '@locus/agent/server/provider-extension';
import { discoverModels, fallbackModels } from './models.js';
export const ohmypiProvider: AgentProviderExtension = {
  command: 'omp',
  discoverModels,
  fallbackModels,
  id: 'ohmypi',
  name: 'Oh My Pi',
};
