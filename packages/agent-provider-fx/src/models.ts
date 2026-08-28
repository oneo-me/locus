import {
  agentModels,
  readAgentModelCommandOutput,
} from '@locus/agent/server/provider-models';
export const fallbackModels = [];
export async function discoverModels(binary: string) {
  const value = JSON.parse(
    await readAgentModelCommandOutput(binary, ['models', '--json']),
  ) as { ids?: unknown };
  return Array.isArray(value.ids)
    ? agentModels(
        value.ids.filter((id): id is string => typeof id === 'string'),
      )
    : [];
}
