import { execFile } from 'node:child_process';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { promisify, stripVTControlCharacters } from 'node:util';
import { mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import type {
  AgentProviderId,
  AgentProviderModel,
  AgentProviderModelOption,
} from '../contracts/provider.js';

const REASONING_LEVELS = [
  'none',
  'minimal',
  'low',
  'medium',
  'high',
  'xhigh',
  'max',
] as const;

const execute = promisify(execFile);

const FALLBACK_MODELS: Partial<Record<AgentProviderId, AgentProviderModel[]>> =
  {
    amp: models(['low', 'medium', 'high', 'ultra'], 'medium'),
    claude: claudeModels(),
    codex: models(
      ['gpt-5.6-sol', 'gpt-5.6-terra', 'gpt-5.6-luna', 'gpt-5.5', 'gpt-5.4'],
      'gpt-5.6-sol',
      reasoningOptions(['low', 'medium', 'high', 'xhigh']),
      'medium',
    ),
    cursor: models(['auto'], 'auto'),
    grok: models(
      ['grok-build'],
      'grok-build',
      reasoningOptions(['low', 'medium', 'high', 'xhigh']),
      'high',
    ),
    opencode: models(['default'], 'default'),
    pi: models(['default'], 'default'),
  };

export function fallbackAgentProviderModels(
  providerId: AgentProviderId,
): AgentProviderModel[] {
  return (FALLBACK_MODELS[providerId] ?? []).map((model) => ({ ...model }));
}

export async function discoverAgentProviderModels(
  providerId: AgentProviderId,
  path: string,
  cacheDirectory?: string,
): Promise<AgentProviderModel[]> {
  const discovered = await discover(providerId, path).catch(() => []);
  if (discovered.length > 0) {
    const models = deduplicate(discovered);
    if (cacheDirectory)
      await writeModelCache(cacheDirectory, providerId, models);
    return models;
  }
  return (
    (cacheDirectory
      ? await readModelCache(cacheDirectory, providerId)
      : null) ?? fallbackAgentProviderModels(providerId)
  );
}

async function readModelCache(
  directory: string,
  providerId: AgentProviderId,
): Promise<AgentProviderModel[] | null> {
  try {
    const value = JSON.parse(
      await readFile(join(directory, `${providerId}.json`), 'utf8'),
    ) as unknown;
    if (!Array.isArray(value)) return null;
    const models = value.filter(isAgentProviderModel);
    return models.length > 0 ? models : null;
  } catch {
    return null;
  }
}

async function writeModelCache(
  directory: string,
  providerId: AgentProviderId,
  models: AgentProviderModel[],
): Promise<void> {
  try {
    await mkdir(directory, { recursive: true });
    const path = join(directory, `${providerId}.json`);
    const temporary = `${path}.tmp`;
    await writeFile(temporary, JSON.stringify(models));
    await rename(temporary, path);
  } catch {
    return;
  }
}

function isAgentProviderModel(value: unknown): value is AgentProviderModel {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as AgentProviderModel).id === 'string' &&
    typeof (value as AgentProviderModel).name === 'string' &&
    typeof (value as AgentProviderModel).isDefault === 'boolean' &&
    Array.isArray((value as AgentProviderModel).reasoningEfforts) &&
    ((value as AgentProviderModel).defaultReasoningEffort === null ||
      typeof (value as AgentProviderModel).defaultReasoningEffort === 'string')
  );
}

async function discover(
  providerId: AgentProviderId,
  path: string,
): Promise<AgentProviderModel[]> {
  if (providerId === 'codex') {
    return discoverCodex(path);
  }
  if (providerId === 'cursor') {
    return parseLines(await output(path, ['models']));
  }
  if (providerId === 'opencode') {
    return parseLines(await output(path, ['models'])).filter((model) =>
      model.id.includes('/'),
    );
  }
  if (providerId === 'grok') {
    return parseGrok(await output(path, ['models']));
  }
  if (providerId === 'fx') {
    const value = JSON.parse(await output(path, ['models', '--json'])) as {
      ids?: unknown;
    };
    return Array.isArray(value.ids)
      ? models(value.ids.filter((id): id is string => typeof id === 'string'))
      : [];
  }
  if (providerId === 'kimi') {
    const value = JSON.parse(
      await output(path, ['provider', 'list', '--json']),
    ) as { models?: unknown };
    return value.models && typeof value.models === 'object'
      ? models(Object.keys(value.models))
      : [];
  }
  if (providerId === 'pi' || providerId === 'ohmypi') {
    return discoverPi(path, providerId);
  }
  return fallbackAgentProviderModels(providerId);
}

async function discoverCodex(path: string): Promise<AgentProviderModel[]> {
  const child = spawn(path, ['app-server', '--stdio'], {
    stdio: ['pipe', 'pipe', 'ignore'],
  });
  return new Promise((resolve) => {
    let requestId = 0;
    let settled = false;
    const discovered: AgentProviderModel[] = [];
    const lines = createInterface({ input: child.stdout });
    const timeout = setTimeout(() => finish(discovered), 5_000);
    const finish = (result: AgentProviderModel[]) => {
      if (settled) return;
      settled = true;
      clearTimeout(timeout);
      lines.close();
      child.kill();
      resolve(result);
    };
    const send = (value: unknown) => {
      child.stdin.write(`${JSON.stringify(value)}\n`, (error) => {
        if (error) finish(discovered);
      });
    };

    child.once('error', () => finish(discovered));
    child.stdin.once('error', () => finish(discovered));
    lines.on('line', (line) => {
      let value: {
        id?: unknown;
        result?: {
          data?: unknown;
          nextCursor?: unknown;
        };
      };
      try {
        value = JSON.parse(line) as typeof value;
      } catch {
        return;
      }
      if (value.id === 0) {
        send({ method: 'initialized', params: {} });
        requestId = 1;
        send({ id: requestId, method: 'model/list', params: {} });
        return;
      }
      if (value.id !== requestId || !Array.isArray(value.result?.data)) return;
      discovered.push(...parseCodexModels(value.result.data));
      const cursor = value.result.nextCursor;
      if (typeof cursor !== 'string' || !cursor || requestId >= 32) {
        finish(discovered);
        return;
      }
      requestId += 1;
      send({
        id: requestId,
        method: 'model/list',
        params: { cursor },
      });
    });
    send({
      id: 0,
      method: 'initialize',
      params: {
        capabilities: { experimentalApi: true },
        clientInfo: { name: 'locus', title: 'Locus', version: '0.1.0' },
      },
    });
  });
}

function parseCodexModels(values: unknown[]): AgentProviderModel[] {
  return values.flatMap((value) => {
    if (!value || typeof value !== 'object') return [];
    const candidate = value as Record<string, unknown>;
    if (typeof candidate.model !== 'string') return [];
    const defaultValue = candidate.isDefault;
    return [
      {
        defaultReasoningEffort:
          typeof candidate.defaultReasoningEffort === 'string'
            ? candidate.defaultReasoningEffort
            : null,
        id: candidate.model,
        isDefault:
          defaultValue === true ||
          (typeof defaultValue === 'object' &&
            defaultValue !== null &&
            (defaultValue as Record<string, unknown>).isDefault === true),
        name:
          typeof candidate.displayName === 'string' && candidate.displayName
            ? candidate.displayName
            : displayName(candidate.model),
        reasoningEfforts: Array.isArray(candidate.supportedReasoningEfforts)
          ? candidate.supportedReasoningEfforts.flatMap((value) => {
              if (!value || typeof value !== 'object') return [];
              const effort = (value as Record<string, unknown>).reasoningEffort;
              return typeof effort === 'string'
                ? [{ id: effort, name: reasoningLabel(effort) }]
                : [];
            })
          : [],
      },
    ];
  });
}

async function output(path: string, args: string[]): Promise<string> {
  const result = await execute(path, args, {
    maxBuffer: 4 * 1024 * 1024,
    timeout: 5_000,
  });
  return `${result.stdout}\n${result.stderr}`;
}

async function discoverPi(
  path: string,
  providerId: 'ohmypi' | 'pi',
): Promise<AgentProviderModel[]> {
  const args =
    providerId === 'pi'
      ? [
          '--mode',
          'rpc',
          '--no-session',
          '--no-skills',
          '--no-prompt-templates',
          '--no-context-files',
        ]
      : [
          '--mode',
          'rpc',
          '--no-session',
          '--no-skills',
          '--no-rules',
          '--no-extensions',
        ];
  const child = spawn(path, args, {
    env:
      providerId === 'pi'
        ? { ...process.env, PI_SKIP_VERSION_CHECK: '1' }
        : process.env,
    stdio: ['pipe', 'pipe', 'ignore'],
  });
  return new Promise((resolve) => {
    let settled = false;
    let available: Array<{
      id?: unknown;
      name?: unknown;
      provider?: unknown;
      reasoning?: unknown;
      thinkingLevelMap?: unknown;
    }> | null = null;
    let state: {
      model?: { id?: unknown; provider?: unknown };
      thinkingLevel?: unknown;
    } | null = null;
    const timeout = setTimeout(() => finish(piModels(available, state)), 5_000);
    const lines = createInterface({ input: child.stdout });
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
          data?: {
            models?: Array<{
              id?: unknown;
              name?: unknown;
              provider?: unknown;
              reasoning?: unknown;
              thinkingLevelMap?: unknown;
            }>;
            model?: { id?: unknown; provider?: unknown };
            thinkingLevel?: unknown;
          };
          id?: unknown;
        };
        if (value.id === 'locus-state') {
          state = value.data ?? null;
          if (available) finish(piModels(available, state));
        } else if (
          value.id === 'locus-models' &&
          Array.isArray(value.data?.models)
        ) {
          available = value.data.models;
          child.stdin.write(
            `${JSON.stringify({ id: 'locus-state', type: 'get_state' })}\n`,
          );
        } else {
          return;
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

function piModels(
  available: Array<{
    id?: unknown;
    name?: unknown;
    provider?: unknown;
    reasoning?: unknown;
    thinkingLevelMap?: unknown;
  }> | null,
  state: {
    model?: { id?: unknown; provider?: unknown };
    thinkingLevel?: unknown;
  } | null,
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
    if (typeof model.id !== 'string' || typeof model.provider !== 'string') {
      return [];
    }
    const id = `${model.provider}/${model.id}`;
    const efforts = piReasoningOptions(model);
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

function parseLines(value: string): AgentProviderModel[] {
  return models(
    stripVTControlCharacters(value)
      .split('\n')
      .map((line) =>
        line
          .trim()
          .replace(/^[-*•]\s*/, '')
          .replace(/\s+\(default\)$/i, ''),
      )
      .map((line) => line.split(/\t|\s{2,}|\s+-\s+/)[0]?.trim())
      .filter(
        (line) =>
          line &&
          !line.endsWith(':') &&
          line.split(/\s+/).length === 1 &&
          !line.toLowerCase().includes('error'),
      ),
  );
}

function parseGrok(value: string): AgentProviderModel[] {
  const lines = stripVTControlCharacters(value).split('\n');
  const start = lines.findIndex((line) => line.trim() === 'Available models:');
  if (start < 0) return [];
  return parseLines(lines.slice(start + 1).join('\n')).map((model) => ({
    ...model,
    defaultReasoningEffort: 'high',
    reasoningEfforts: reasoningOptions(['low', 'medium', 'high', 'xhigh']),
  }));
}

function models(
  ids: string[],
  defaultId?: string,
  reasoningEfforts: AgentProviderModelOption[] = [],
  defaultReasoningEffort: string | null = null,
): AgentProviderModel[] {
  return ids
    .map((id) => id.trim())
    .filter(Boolean)
    .map((id) => ({
      defaultReasoningEffort,
      id,
      isDefault: id === defaultId,
      name: displayName(id),
      reasoningEfforts,
    }));
}

function claudeModels(): AgentProviderModel[] {
  const ultracode = reasoningOptions([
    'low',
    'medium',
    'high',
    'xhigh',
    'max',
    'ultracode',
  ]);
  const standard = reasoningOptions(['low', 'medium', 'high', 'xhigh', 'max']);
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
    const supportsUltracode =
      id === 'claude-fable-5' ||
      id === 'claude-opus-5' ||
      id === 'claude-opus-4-8' ||
      id === 'claude-opus-4-7' ||
      id === 'claude-sonnet-5';
    const reasoningEfforts =
      id === 'claude-haiku-4-5' ? [] : supportsUltracode ? ultracode : standard;
    return {
      defaultReasoningEffort: reasoningEfforts.length ? 'high' : null,
      id,
      isDefault: id === 'claude-sonnet-5',
      name: displayName(id),
      reasoningEfforts,
    };
  });
}

function piReasoningOptions(model: {
  reasoning?: unknown;
  thinkingLevelMap?: unknown;
}): AgentProviderModelOption[] {
  if (model.reasoning !== true) return [];
  const levelMap =
    model.thinkingLevelMap && typeof model.thinkingLevelMap === 'object'
      ? (model.thinkingLevelMap as Record<string, unknown>)
      : null;
  return reasoningOptions(
    REASONING_LEVELS.filter((level) => {
      const mapped = levelMap?.[level];
      return level === 'xhigh' || level === 'max'
        ? mapped !== undefined && mapped !== null
        : mapped !== null;
    }),
  );
}

function reasoningOptions(ids: readonly string[]): AgentProviderModelOption[] {
  return ids.map((id) => ({ id, name: reasoningLabel(id) }));
}

function reasoningLabel(id: string): string {
  return (
    (
      {
        high: '高',
        low: '低',
        max: '最高',
        medium: '中',
        minimal: '最低',
        none: '关闭',
        ultracode: 'Ultracode',
        xhigh: '超高',
      } as Record<string, string>
    )[id] ?? displayName(id)
  );
}

function displayName(id: string): string {
  const segment = id.split('/').at(-1) ?? id;
  return segment
    .split(/[-_]/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

function deduplicate(values: AgentProviderModel[]): AgentProviderModel[] {
  return [...new Map(values.map((model) => [model.id, model])).values()];
}
