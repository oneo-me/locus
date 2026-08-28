import {
  agentModels,
  parseAgentModelLines,
  readAgentModelCommandOutput,
} from '@locus/agent/server/provider-models';
export const fallbackModels = agentModels(['default'], 'default');
export async function discoverModels(binary: string) {
  return parseAgentModelLines(
    await readAgentModelCommandOutput(binary, ['models']),
  ).filter((model) => model.id.includes('/'));
}
