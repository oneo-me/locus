import {
  agentModels,
  agentReasoningOptions,
  parseAgentModelLines,
  readAgentModelCommandOutput,
} from '@locus/agent/server/provider-models';
export const fallbackModels = agentModels(
  ['grok-build'],
  'grok-build',
  agentReasoningOptions(['low', 'medium', 'high', 'xhigh']),
  'high',
);
export async function discoverModels(binary: string) {
  const lines = (await readAgentModelCommandOutput(binary, ['models'])).split(
    '\n',
  );
  const start = lines.findIndex((line) => line.trim() === 'Available models:');
  if (start < 0) return [];
  return parseAgentModelLines(lines.slice(start + 1).join('\n')).map(
    (model) => ({
      ...model,
      defaultReasoningEffort: 'high',
      reasoningEfforts: agentReasoningOptions([
        'low',
        'medium',
        'high',
        'xhigh',
      ]),
    }),
  );
}
