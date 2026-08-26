import type { AgentProviderId } from './provider.js';

export interface AgentProject {
  id: string;
  name: string;
  sourcePath: string;
}

export interface AgentProjectList {
  projects: AgentProject[];
}

export interface AgentSessionMode {
  id: string;
  name: string;
}

export interface AgentSessionModeState {
  available: AgentSessionMode[];
  currentId: string;
}

interface AgentSessionConfigBase {
  category: string | null;
  id: string;
  name: string;
}

export interface AgentSessionBooleanConfig extends AgentSessionConfigBase {
  currentValue: boolean;
  type: 'boolean';
}

export interface AgentSessionSelectConfig extends AgentSessionConfigBase {
  currentValue: string;
  options: Array<{ name: string; value: string }>;
  type: 'select';
}

export type AgentSessionConfig =
  | AgentSessionBooleanConfig
  | AgentSessionSelectConfig;

export type AgentConversationStatus =
  | 'idle'
  | 'connecting'
  | 'working'
  | 'waiting'
  | 'failed';

export const DEFAULT_AGENT_CONVERSATION_TITLE = '新任务';

export function agentConversationCanDelete(
  status: AgentConversationStatus,
): boolean {
  return status !== 'connecting' && status !== 'working';
}

export function agentConversationDisplayTitle(
  conversation: Pick<AgentConversation, 'autoTitle' | 'title'>,
): string {
  const title = conversation.title.trim();
  if (title && title !== DEFAULT_AGENT_CONVERSATION_TITLE) return title;
  return conversation.autoTitle?.trim() || DEFAULT_AGENT_CONVERSATION_TITLE;
}

export function agentConversationTitleFromPrompt(content: string): string {
  const title = content.split(/\s+/).slice(0, 7).join(' ');
  const characters = Array.from(title);
  return characters.length > 54
    ? `${characters.slice(0, 53).join('')}…`
    : title;
}

export interface AgentMessageEntry {
  content: string;
  createdAt: string;
  id: string;
  kind: 'message';
  role: 'assistant' | 'user';
  streaming: boolean;
}

export interface AgentThoughtEntry {
  content: string;
  createdAt: string;
  id: string;
  kind: 'thought';
  streaming: boolean;
}

export interface AgentToolEntry {
  content: string | null;
  createdAt: string;
  id: string;
  kind: 'tool';
  status: 'completed' | 'failed' | 'in_progress' | 'pending';
  title: string;
  toolKind: string | null;
}

export interface AgentPlanEntry {
  content: string;
  priority: 'high' | 'low' | 'medium';
  status: 'completed' | 'in_progress' | 'pending';
}

export interface AgentPlanTranscriptEntry {
  createdAt: string;
  entries: AgentPlanEntry[];
  id: string;
  kind: 'plan';
}

export interface AgentPermissionOption {
  id: string;
  kind: string;
  name: string;
}

export interface AgentPermissionEntry {
  createdAt: string;
  id: string;
  kind: 'permission';
  options: AgentPermissionOption[];
  requestId: string;
  selectedOptionId: string | null;
  status: 'cancelled' | 'pending' | 'selected';
  title: string;
}

export interface AgentErrorEntry {
  content: string;
  createdAt: string;
  id: string;
  kind: 'error';
}

export type AgentTranscriptEntry =
  | AgentErrorEntry
  | AgentMessageEntry
  | AgentPermissionEntry
  | AgentPlanTranscriptEntry
  | AgentThoughtEntry
  | AgentToolEntry;

export interface AgentConversation {
  acpSessionId: string | null;
  activityAt: string;
  autoTitle: string | null;
  availableCommands: Array<{ description: string; name: string }>;
  configOptions: AgentSessionConfig[];
  contextTokens: number | null;
  contextWindow: number | null;
  createdAt: string;
  cwd: string;
  id: string;
  modes: AgentSessionModeState | null;
  projectId: string;
  projectName: string;
  providerId: AgentProviderId;
  status: AgentConversationStatus;
  title: string;
  transcript: AgentTranscriptEntry[];
  updatedAt: string;
}

export interface AgentConversationSummary {
  activityAt: string;
  autoTitle: string | null;
  createdAt: string;
  id: string;
  projectId: string;
  projectName: string;
  providerId: AgentProviderId;
  status: AgentConversationStatus;
  title: string;
  updatedAt: string;
}

export interface AgentConversationList {
  conversations: AgentConversationSummary[];
}

export interface CreateAgentConversationInput {
  projectId: string;
  providerId?: AgentProviderId;
}

export interface SendAgentMessageInput {
  content: string;
}

export interface RespondAgentPermissionInput {
  optionId: string;
}

export interface SetAgentModeInput {
  modeId: string;
}

export interface SetAgentConfigInput {
  configId: string;
  value: boolean | string;
}

export interface RenameAgentConversationInput {
  title: string;
}

export interface AgentConversationEvent {
  conversation: AgentConversation;
  type: 'conversation';
}

export interface AgentConversationSnapshotEvent {
  conversations: AgentConversationSummary[];
  type: 'snapshot';
}

export type AgentWorkspaceEvent =
  | AgentConversationEvent
  | AgentConversationSnapshotEvent;
