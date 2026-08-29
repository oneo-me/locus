import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import type {
  AgentProviderModel,
  AgentProviderModelOption,
} from '../../contracts/provider.js';
import {
  agentModels,
  agentReasoningOptions,
} from '../../server/provider-models.js';

const reasoningLevels = [
  'none',
  'minimal',
  'low',
  'medium',
  'high',
  'xhigh',
  'max',
] as const;
export const fallbackModels = agentModels(['default'], 'default');

export async function discoverModels(
  path: string,
): Promise<AgentProviderModel[]> {
  const child = spawn(
    path,
    [
      '--mode',
      'rpc',
      '--no-session',
      '--no-skills',
      '--no-prompt-templates',
      '--no-context-files',
    ],
    {
      env: { ...process.env, PI_SKIP_VERSION_CHECK: '1' },
      stdio: ['pipe', 'pipe', 'ignore'],
    },
  );
  return new Promise((resolve) => {
    let settled = false;
    let available: Array<ModelValue> | null = null;
    let state: StateValue | null = null;
    const lines = createInterface({ input: child.stdout });
    const timeout = setTimeout(() => finish(toModels(available, state)), 5_000);
    const finish = (result: AgentProviderModel[]) => {
      if (settled) return;
      settled = true;
      clearTimeout(timeout);
      lines.close();
      child.kill();
      resolve(result);
    };
    child.once('error', () => finish([]));
    lines.on('line', (line) => {
      try {
        const value = JSON.parse(line) as {
          data?: StateValue & { models?: Array<ModelValue> };
          id?: unknown;
        };
        if (value.id === 'locus-state') {
          state = value.data ?? null;
          if (available) finish(toModels(available, state));
        } else if (
          value.id === 'locus-models' &&
          Array.isArray(value.data?.models)
        ) {
          available = value.data.models;
          child.stdin.write(
            `${JSON.stringify({ id: 'locus-state', type: 'get_state' })}\n`,
          );
        }
      } catch {
        return;
      }
    });
    child.stdin.write(
      `${JSON.stringify({ id: 'locus-models', type: 'get_available_models' })}\n`,
    );
  });
}

interface ModelValue {
  id?: unknown;
  name?: unknown;
  provider?: unknown;
  reasoning?: unknown;
  thinkingLevelMap?: unknown;
}
interface StateValue {
  model?: { id?: unknown; provider?: unknown };
  thinkingLevel?: unknown;
}

function toModels(
  available: Array<ModelValue> | null,
  state: StateValue | null,
): AgentProviderModel[] {
  if (!available) return [];
  const defaultId =
    typeof state?.model?.provider === 'string' &&
    typeof state.model.id === 'string'
      ? `${state.model.provider}/${state.model.id}`
      : null;
  const currentThinking =
    typeof state?.thinkingLevel === 'string' ? state.thinkingLevel : null;
  return available.flatMap((model) => {
    if (typeof model.id !== 'string' || typeof model.provider !== 'string')
      return [];
    const id = `${model.provider}/${model.id}`;
    const efforts = reasoningOptions(model);
    const defaultReasoningEffort = efforts.some(
      (effort) => effort.id === currentThinking,
    )
      ? currentThinking
      : efforts.some((effort) => effort.id === 'medium')
        ? 'medium'
        : (efforts[0]?.id ?? null);
    return [
      {
        defaultReasoningEffort,
        id,
        isDefault: id === defaultId,
        name: typeof model.name === 'string' ? model.name : model.id,
        reasoningEfforts: efforts,
      },
    ];
  });
}

function reasoningOptions(model: ModelValue): AgentProviderModelOption[] {
  if (model.reasoning !== true) return [];
  const levelMap =
    model.thinkingLevelMap && typeof model.thinkingLevelMap === 'object'
      ? (model.thinkingLevelMap as Record<string, unknown>)
      : null;
  return agentReasoningOptions(
    reasoningLevels.filter((level) => {
      const mapped = levelMap?.[level];
      return level === 'xhigh' || level === 'max'
        ? mapped !== undefined && mapped !== null
        : mapped !== null;
    }),
  );
}
