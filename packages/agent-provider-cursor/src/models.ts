import {
  agentModels,
  parseAgentModelLines,
  readAgentModelCommandOutput,
} from '@locus/agent/server/provider-models';
export const fallbackModels = agentModels(['auto'], 'auto');
export async function discoverModels(binary: string) {
  return parseAgentModelLines(
    await readAgentModelCommandOutput(binary, ['models']),
  );
}
