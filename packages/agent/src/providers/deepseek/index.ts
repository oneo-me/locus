import type { AgentProvider } from '../protocol.js';
import { discoverModels, fallbackModels } from './models.js';
export const deepseekProvider: AgentProvider = {
  command: 'dsh',
  discoverModels,
  fallbackModels,
  id: 'deepseek',
  name: 'DeepSeek Harness',
};
