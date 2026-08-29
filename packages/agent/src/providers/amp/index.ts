import type { AgentProvider } from '../protocol.js';
import { discoverModels, fallbackModels } from './models.js';
export const ampProvider: AgentProvider = {
  command: 'amp',
  discoverModels,
  fallbackModels,
  id: 'amp',
  name: 'Amp',
};
