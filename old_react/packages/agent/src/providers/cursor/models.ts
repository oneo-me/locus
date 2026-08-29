import {
  agentModels,
  parseAgentModelLines,
  readAgentModelCommandOutput,
} from '../../server/provider-models.js';
export const fallbackModels = agentModels(['auto'], 'auto');
export async function discoverModels(binary: string) {
  return parseAgentModelLines(
    await readAgentModelCommandOutput(binary, ['models']),
  );
}
