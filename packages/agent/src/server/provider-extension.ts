import type { SessionNotification } from '@agentclientprotocol/sdk';
import type {
  AgentConversation,
  AgentSessionConfig,
} from '../contracts/conversation.js';
import type {
  AgentProviderId,
  AgentProviderModel,
} from '../contracts/provider.js';
import type {
  AgentRuntime,
  AgentRuntimePermissionRequest,
  NativeAgentRuntimeHandlers,
} from './agent-runtime.js';

export interface AgentProviderRuntimeHandlers {
  onConnected: (
    sessionId: string,
    modes: AgentConversation['modes'],
    configOptions: AgentSessionConfig[],
    sessionPath?: string | null,
  ) => void;
  onError: (message: string) => void;
  onExit: () => void;
  onPermission: (permission: AgentRuntimePermissionRequest) => void;
  onUpdate: (notification: SessionNotification) => void;
  native: NativeAgentRuntimeHandlers;
}

export interface AgentProviderRuntimeOptions {
  binary: string;
  conversation: AgentConversation;
  handlers: AgentProviderRuntimeHandlers;
  models: AgentProviderModel[];
  selectedModel?: string;
  selectedReasoningEffort?: string;
}

export interface AgentProviderExtension {
  command: string;
  createRuntime?: (
    options: AgentProviderRuntimeOptions,
  ) => Promise<AgentRuntime>;
  discoverModels: (binary: string) => Promise<AgentProviderModel[]>;
  fallbackModels: AgentProviderModel[];
  id: AgentProviderId;
  name: string;
  normalizeConfigOptions?: (
    options: AgentSessionConfig[],
  ) => AgentSessionConfig[];
  normalizeTitle?: (title: string | null) => string | null | undefined;
  readSessionTitle?: (sessionId: string) => string | null;
  restartOnConfigChange?: (
    option: AgentSessionConfig,
    value: boolean | string,
  ) => boolean;
}
