import { execFile } from 'node:child_process';
import { mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { promisify, stripVTControlCharacters } from 'node:util';
import type {
  AgentProviderId,
  AgentProviderModel,
  AgentProviderModelOption,
} from '../contracts/provider.js';
import type { AgentProviderExtension } from './provider-extension.js';

const execute = promisify(execFile);

export async function resolveAgentProviderModels(
  provider: AgentProviderExtension,
  path: string,
  cacheDirectory?: string,
): Promise<AgentProviderModel[]> {
  const discovered = await provider.discoverModels(path).catch(() => []);
  if (discovered.length > 0) {
    const models = deduplicate(discovered);
    if (cacheDirectory)
      await writeModelCache(cacheDirectory, provider.id, models);
    return models;
  }
  return (
    (cacheDirectory
      ? await readModelCache(cacheDirectory, provider.id)
      : null) ?? provider.fallbackModels.map((model) => ({ ...model }))
  );
}

export function agentModels(
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
      name: agentModelDisplayName(id),
      reasoningEfforts,
    }));
}

export function agentReasoningOptions(
  ids: readonly string[],
): AgentProviderModelOption[] {
  return ids.map((id) => ({ id, name: agentReasoningLabel(id) }));
}

export function agentReasoningLabel(id: string): string {
  return (
    {
      high: '高',
      low: '低',
      max: '最高',
      medium: '中',
      minimal: '最低',
      none: '关闭',
      ultracode: 'Ultracode',
      xhigh: '超高',
    }[id] ?? agentModelDisplayName(id)
  );
}

export function agentModelDisplayName(id: string): string {
  const segment = id.split('/').at(-1) ?? id;
  return segment
    .split(/[-_]/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

export function parseAgentModelLines(value: string): AgentProviderModel[] {
  return agentModels(
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
        (line): line is string =>
          Boolean(line) &&
          !line.endsWith(':') &&
          line.split(/\s+/).length === 1 &&
          !line.toLowerCase().includes('error'),
      ),
  );
}

export async function readAgentModelCommandOutput(
  path: string,
  args: string[],
): Promise<string> {
  const result = await execute(path, args, {
    maxBuffer: 4 * 1024 * 1024,
    timeout: 5_000,
  });
  return `${result.stdout}\n${result.stderr}`;
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

function deduplicate(values: AgentProviderModel[]): AgentProviderModel[] {
  return [...new Map(values.map((model) => [model.id, model])).values()];
}
