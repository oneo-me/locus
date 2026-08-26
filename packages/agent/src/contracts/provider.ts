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

export type AgentProviderId = (typeof AGENT_PROVIDERS)[number]['id'];

export interface AgentProviderProbe {
  command: string;
  id: AgentProviderId;
  installed: boolean;
  name: string;
  path: string | null;
}

export interface AgentProviderDetection {
  checkedAt: string | null;
  providers: AgentProviderProbe[];
}
