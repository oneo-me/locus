import type {
  AgentConversation,
  AgentConversationList,
  AgentProjectList,
  CreateAgentConversationInput,
  RespondAgentPermissionInput,
  SendAgentMessageInput,
  SetAgentConfigInput,
  SetAgentModeInput,
} from '@locus/agent/contracts/conversation';
import type { AgentProviderDetection } from '@locus/agent/contracts/provider';
import { apiGet, apiSend } from './api-client';

export const taskApi = {
  cancel: (id: string) =>
    apiSend<AgentConversation>(`/api/tasks/${id}/cancel`, 'POST'),
  connect: (id: string) =>
    apiSend<AgentConversation>(`/api/tasks/${id}/connect`, 'POST'),
  create: (input: CreateAgentConversationInput) =>
    apiSend<AgentConversation>('/api/tasks', 'POST', input),
  delete: (id: string) => apiSend<{ ok: true }>(`/api/tasks/${id}`, 'DELETE'),
  get: (id: string) => apiGet<AgentConversation>(`/api/tasks/${id}`),
  list: () => apiGet<AgentConversationList>('/api/tasks'),
  listProjects: () => apiGet<AgentProjectList>('/api/task-projects'),
  providers: () => apiGet<AgentProviderDetection>('/api/agent-providers'),
  respondPermission: (
    id: string,
    requestId: string,
    input: RespondAgentPermissionInput,
  ) =>
    apiSend<AgentConversation>(
      `/api/tasks/${id}/permissions/${requestId}`,
      'POST',
      input,
    ),
  sendMessage: (id: string, input: SendAgentMessageInput) =>
    apiSend<AgentConversation>(`/api/tasks/${id}/messages`, 'POST', input),
  setConfig: (id: string, input: SetAgentConfigInput) =>
    apiSend<AgentConversation>(`/api/tasks/${id}/config`, 'POST', input),
  setMode: (id: string, input: SetAgentModeInput) =>
    apiSend<AgentConversation>(`/api/tasks/${id}/mode`, 'POST', input),
};
