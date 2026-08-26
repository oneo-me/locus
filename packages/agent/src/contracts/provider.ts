export const AGENT_PROVIDERS = [
  { id: 'amp', name: 'Amp', command: 'amp' },
  { id: 'claude', name: 'Claude Code', command: 'claude' },
  { id: 'codex', name: 'Codex CLI', command: 'codex' },
  { id: 'cursor', name: 'Cursor CLI', command: 'cursor-agent' },
  {
    id: 'deepseek',
    name: 'DeepSeek Harness',
    command: 'dsh',
  },
  { id: 'fx', name: 'Fx', command: 'fx' },
  { id: 'opencode', name: 'OpenCode', command: 'opencode' },
  { id: 'grok', name: 'Grok Build', command: 'grok' },
  { id: 'kimi', name: 'Kimi Code', command: 'kimi' },
  { id: 'ohmypi', name: 'Oh My Pi', command: 'omp' },
  { id: 'pi', name: 'Pi', command: 'pi' },
] as const;

export const ACP_AGENT_PROVIDER_IDS = ['cursor', 'grok', 'fx', 'kimi'] as const;

export const TASK_AGENT_PROVIDER_IDS = [
  ...ACP_AGENT_PROVIDER_IDS,
  'opencode',
] as const;

export type AgentProviderId = (typeof AGENT_PROVIDERS)[number]['id'];
export type AcpAgentProviderId = (typeof ACP_AGENT_PROVIDER_IDS)[number];
export type TaskAgentProviderId = (typeof TASK_AGENT_PROVIDER_IDS)[number];

export function isAcpAgentProviderId(
  value: AgentProviderId,
): value is AcpAgentProviderId {
  return (ACP_AGENT_PROVIDER_IDS as readonly string[]).includes(value);
}

export function isTaskAgentProviderId(
  value: AgentProviderId,
): value is TaskAgentProviderId {
  return (TASK_AGENT_PROVIDER_IDS as readonly string[]).includes(value);
}

export interface AgentProviderProbe {
  command: string;
  id: AgentProviderId;
  installed: boolean;
  models: AgentProviderModel[];
  name: string;
  path: string | null;
}

export interface AgentProviderModel {
  id: string;
  isDefault: boolean;
  name: string;
}

export interface AgentProviderDetection {
  checkedAt: string | null;
  providers: AgentProviderProbe[];
}
