import { agentModels } from '../../server/provider-models.js';
export const fallbackModels = agentModels(
  ['low', 'medium', 'high', 'ultra'],
  'medium',
);
export async function discoverModels() {
  return [];
}
