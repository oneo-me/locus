export type AgentProviderId = string;

export interface AgentProviderProbe {
  command: string;
  id: AgentProviderId;
  installed: boolean;
  models: AgentProviderModel[];
  name: string;
  path: string | null;
  supportsTasks: boolean;
}

export interface AgentProviderModel {
  defaultReasoningEffort: string | null;
  id: string;
  isDefault: boolean;
  name: string;
  reasoningEfforts: AgentProviderModelOption[];
}

export interface AgentProviderModelOption {
  id: string;
  name: string;
}

export interface AgentProviderDetection {
  checkedAt: string | null;
  providers: AgentProviderProbe[];
}
