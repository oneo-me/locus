import {
  agentModelDisplayName,
  agentReasoningOptions,
} from '@locus/agent/server/provider-models';
import type { AgentProviderModel } from '@locus/agent/contracts/provider';
export const fallbackModels: AgentProviderModel[] = claudeModels();
export async function discoverModels() {
  return [];
}
function claudeModels(): AgentProviderModel[] {
  const ultracode = agentReasoningOptions([
    'low',
    'medium',
    'high',
    'xhigh',
    'max',
    'ultracode',
  ]);
  const standard = agentReasoningOptions([
    'low',
    'medium',
    'high',
    'xhigh',
    'max',
  ]);
  return [
    'claude-fable-5',
    'claude-opus-5',
    'claude-opus-4-8',
    'claude-opus-4-7',
    'claude-opus-4-6',
    'claude-opus-4-5',
    'claude-sonnet-5',
    'claude-sonnet-4-6',
    'claude-haiku-4-5',
  ].map((id) => {
    const extended = [
      'claude-fable-5',
      'claude-opus-5',
      'claude-opus-4-8',
      'claude-opus-4-7',
      'claude-sonnet-5',
    ].includes(id);
    const reasoningEfforts =
      id === 'claude-haiku-4-5' ? [] : extended ? ultracode : standard;
    return {
      defaultReasoningEffort: reasoningEfforts.length ? 'high' : null,
      id,
      isDefault: id === 'claude-sonnet-5',
      name: agentModelDisplayName(id),
      reasoningEfforts,
    };
  });
}
