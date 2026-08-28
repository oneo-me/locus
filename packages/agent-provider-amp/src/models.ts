import { agentModels } from '@locus/agent/server/provider-models';
export const fallbackModels = agentModels(
  ['low', 'medium', 'high', 'ultra'],
  'medium',
);
export async function discoverModels() {
  return [];
}
