import type { AgentSessionConfig } from '../contracts/conversation.js';
import type { AgentProviderModel } from '../contracts/provider.js';

export const MODEL_CONFIG_ID = 'model';
export const REASONING_EFFORT_CONFIG_ID = 'reasoning_effort';

export function modelConfigOptions(
  models: AgentProviderModel[],
  modelId: string | undefined,
  reasoningEffort?: string,
): AgentSessionConfig[] {
  if (!modelId || models.length === 0) return [];
  const model = models.find((candidate) => candidate.id === modelId);
  const options: AgentSessionConfig[] = [
    {
      category: 'model',
      currentValue: modelId,
      id: MODEL_CONFIG_ID,
      name: '模型',
      options: models.map(({ id, name }) => ({ name, value: id })),
      type: 'select',
    },
  ];
  if (!model?.reasoningEfforts.length) return options;
  const currentValue = model.reasoningEfforts.some(
    (option) => option.id === reasoningEffort,
  )
    ? reasoningEffort
    : (model.defaultReasoningEffort ?? model.reasoningEfforts[0]?.id);
  if (!currentValue) return options;
  options.push({
    category: 'thought_level',
    currentValue,
    id: REASONING_EFFORT_CONFIG_ID,
    name: '推理强度',
    options: model.reasoningEfforts.map(({ id, name }) => ({
      name,
      value: id,
    })),
    type: 'select',
  });
  return options;
}

export function selectedReasoningEffort(
  options: AgentSessionConfig[],
): string | undefined {
  const option = options.find(
    (candidate) =>
      candidate.type === 'select' && candidate.category === 'thought_level',
  );
  return option?.type === 'select' ? option.currentValue : undefined;
}
