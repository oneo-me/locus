import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import type { AgentProviderModel } from '@locus/agent/contracts/provider';
import {
  agentModelDisplayName,
  agentModels,
  agentReasoningLabel,
  agentReasoningOptions,
} from '@locus/agent/server/provider-models';

export const fallbackModels = agentModels(
  ['gpt-5.6-sol', 'gpt-5.6-terra', 'gpt-5.6-luna', 'gpt-5.5', 'gpt-5.4'],
  'gpt-5.6-sol',
  agentReasoningOptions(['low', 'medium', 'high', 'xhigh']),
  'medium',
);

export async function discoverModels(
  path: string,
): Promise<AgentProviderModel[]> {
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
        result?: { data?: unknown; nextCursor?: unknown };
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
      discovered.push(...parseModels(value.result.data));
      const cursor = value.result.nextCursor;
      if (typeof cursor !== 'string' || !cursor || requestId >= 32) {
        finish(discovered);
        return;
      }
      requestId += 1;
      send({ id: requestId, method: 'model/list', params: { cursor } });
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

function parseModels(values: unknown[]): AgentProviderModel[] {
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
            : agentModelDisplayName(candidate.model),
        reasoningEfforts: Array.isArray(candidate.supportedReasoningEfforts)
          ? candidate.supportedReasoningEfforts.flatMap((value) => {
              if (!value || typeof value !== 'object') return [];
              const effort = (value as Record<string, unknown>).reasoningEffort;
              return typeof effort === 'string'
                ? [{ id: effort, name: agentReasoningLabel(effort) }]
                : [];
            })
          : [],
      },
    ];
  });
}
