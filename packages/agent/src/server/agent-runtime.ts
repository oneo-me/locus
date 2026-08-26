import type {
  AgentSessionConfig,
  AgentSessionModeState,
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
