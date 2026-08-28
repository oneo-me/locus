import {
  agentModels,
  readAgentModelCommandOutput,
} from '@locus/agent/server/provider-models';
export const fallbackModels = [];
export async function discoverModels(binary: string) {
  const value = JSON.parse(
    await readAgentModelCommandOutput(binary, ['provider', 'list', '--json']),
  ) as { models?: unknown };
  return value.models && typeof value.models === 'object'
    ? agentModels(Object.keys(value.models))
    : [];
}
