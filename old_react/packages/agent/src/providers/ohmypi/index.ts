import type { AgentProvider } from '../protocol.js';
import { discoverModels, fallbackModels } from './models.js';
export const ohmypiProvider: AgentProvider = {
  command: 'omp',
  discoverModels,
  fallbackModels,
  id: 'ohmypi',
  name: 'Oh My Pi',
};
