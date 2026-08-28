import type { AgentProviderExtension } from '@locus/agent/server/provider-extension';
import { discoverModels, fallbackModels } from './models.js';
export const ampProvider: AgentProviderExtension = {
  command: 'amp',
  discoverModels,
  fallbackModels,
  id: 'amp',
  name: 'Amp',
};
