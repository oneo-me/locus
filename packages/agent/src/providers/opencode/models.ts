import {
  agentModels,
  parseAgentModelLines,
  readAgentModelCommandOutput,
} from '../../server/provider-models.js';
export const fallbackModels = agentModels(['default'], 'default');
export async function discoverModels(binary: string) {
  return parseAgentModelLines(
    await readAgentModelCommandOutput(binary, ['models']),
  ).filter((model) => model.id.includes('/'));
}
