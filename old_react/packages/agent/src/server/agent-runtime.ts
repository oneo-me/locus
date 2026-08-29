import type {
  AgentSessionConfig,
  AgentSessionModeState,
  AgentToolEntry,
} from '../contracts/conversation.js';

export interface AgentRuntime {
  cancel(): Promise<void>;
  close(): void;
  prompt(content: string): Promise<string>;
  respondPermission(requestId: string, optionId: string | null): boolean;
  setConfig(
    configId: string,
    value: boolean | string,
  ): Promise<AgentSessionConfig[]>;
  setMode(modeId: string): Promise<void>;
  setModel(
    configId: string,
    modelId: string,
    configOptions: AgentSessionConfig[],
  ): Promise<AgentSessionConfig[]>;
}

export interface AgentRuntimePermissionRequest {
  options: Array<{ id: string; kind: string; name: string }>;
  requestId: string;
  title: string;
}

export interface AgentRuntimeConnection {
  configOptions: AgentSessionConfig[];
  modes: AgentSessionModeState | null;
  sessionId: string;
}

export interface NativeAgentRuntimeHandlers {
  onCommands: (commands: Array<{ description: string; name: string }>) => void;
  onConnected: (
    sessionId: string,
    configOptions: AgentSessionConfig[],
    sessionPath?: string | null,
  ) => void;
  onError: (message: string) => void;
  onExit: () => void;
  onIdle: () => void;
  onPermission: (permission: AgentRuntimePermissionRequest) => void;
  onText: (kind: 'message' | 'thought', text: string) => void;
  onTitle: (title: string) => void;
  onTool: (tool: Omit<AgentToolEntry, 'createdAt' | 'kind'>) => void;
  onUsage: (tokens: number | null, window: number | null) => void;
}
