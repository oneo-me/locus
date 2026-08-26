import type {
  AgentConversation,
  AgentConversationSummary,
  AgentProject,
  AgentWorkspaceEvent,
} from '@locus/agent/contracts/conversation';
import {
  isTaskAgentProviderId,
  type AgentProviderDetection,
  type AgentProviderId,
} from '@locus/agent/contracts/provider';
import { type CSSProperties, useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
import { useGlobalSettings } from '~/features/settings/global-settings-provider';
import { taskApi } from '~/lib/task-api';
import { AgentConversationView } from './conversation';
import { TasksSidebar } from './sidebar';
import { TasksSidebarResizeHandle } from './sidebar-resize-handle';

interface TasksInitialData {
  conversation: AgentConversation | null;
  conversations: AgentConversationSummary[];
  projects: AgentProject[];
  providers: AgentProviderDetection;
}

export function TasksShell({ initialData }: { initialData: TasksInitialData }) {
  const navigate = useNavigate();
  const { settings, updateSettings } = useGlobalSettings();
  const routeConversationId = initialData.conversation?.id ?? null;
  const availableProviders = initialData.providers.providers.filter(
    (provider) =>
      provider.installed &&
      isTaskAgentProviderId(provider.id) &&
      provider.models.length > 0,
  );
  const preferredProvider = availableProviders.find(
    (provider) => provider.id === settings.taskDraft.providerId,
  );
  const defaultProvider = preferredProvider ?? availableProviders[0];
  const preferredModel = defaultProvider?.models.find(
    (model) =>
      defaultProvider.id === settings.taskDraft.providerId &&
      model.id === settings.taskDraft.modelId,
  );
  const defaultProject =
    initialData.projects.find(
      (project) => project.id === settings.taskDraft.projectId,
    ) ?? initialData.projects[0];
  const [sidebarWidth, setSidebarWidth] = useState(272);
  const [conversations, setConversations] = useState(initialData.conversations);
  const [conversation, setConversation] = useState(initialData.conversation);
  const [projectId, setProjectId] = useState(
    initialData.conversation?.projectId ?? defaultProject?.id ?? null,
  );
  const [providerId, setProviderId] = useState<AgentProviderId | null>(
    initialData.conversation?.providerId ?? defaultProvider?.id ?? null,
  );
  const [modelId, setModelId] = useState<string | null>(
    initialData.conversation
      ? null
      : (preferredModel?.id ??
          defaultProvider?.models.find((model) => model.isDefault)?.id ??
          defaultProvider?.models[0]?.id ??
          null),
  );
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  const persistTaskDraft = (
    nextProjectId: string | null,
    nextProviderId: AgentProviderId | null,
    nextModelId: string | null,
  ) => {
    void updateSettings({
      taskDraft: {
        modelId: nextModelId,
        projectId: nextProjectId,
        providerId: nextProviderId,
      },
    }).catch((cause) => setError(errorMessage(cause)));
  };

  useEffect(() => {
    let active = true;
    const source = new EventSource('/api/task-events');
    source.onmessage = (message) => {
      let event: AgentWorkspaceEvent;
      try {
        event = JSON.parse(message.data) as AgentWorkspaceEvent;
      } catch {
        setError('任务实时事件无效。');
        return;
      }
      if (event.type === 'snapshot') {
        setConversations(event.conversations);
        return;
      }
      const next = event.conversation;
      setConversations((current) => upsertSummary(current, next));
      setConversation((current) => latestConversation(current, next));
    };
    source.onopen = () => {
      setError(null);
      const id = routeConversationId;
      if (!id) return;
      void taskApi.get(id).then(
        (next) => {
          if (!active) return;
          setConversation((current) => latestConversation(current, next));
          setConversations((current) => upsertSummary(current, next));
        },
        (cause) => {
          if (active) setError(errorMessage(cause));
        },
      );
    };
    source.onerror = () => setError('任务实时连接已断开。');
    return () => {
      active = false;
      source.close();
    };
  }, [routeConversationId]);

  const createConversation = () => {
    void navigate('/tasks');
  };

  const runAction = async (action: () => Promise<AgentConversation>) => {
    setPending(true);
    setError(null);
    try {
      const next = await action();
      setConversation(next);
      setConversations((current) => upsertSummary(current, next));
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setPending(false);
    }
  };

  const sendMessage = async (content: string) => {
    if (conversation) {
      await runAction(() => taskApi.sendMessage(conversation.id, { content }));
      return;
    }
    if (!projectId || !providerId) {
      setError('请选择项目和 Agent。');
      return;
    }
    setPending(true);
    setError(null);
    try {
      const created = await taskApi.create({
        projectId,
        providerId,
      });
      setConversation(created);
      setConversations((current) => upsertSummary(current, created));
      const connected = await taskApi.connect(created.id);
      const modelOption = connected.configOptions.find(
        (option) => option.type === 'select' && option.category === 'model',
      );
      if (
        modelId &&
        modelOption?.type === 'select' &&
        modelOption.options.some((option) => option.value === modelId)
      ) {
        await taskApi.setConfig(created.id, {
          configId: modelOption.id,
          value: modelId,
        });
      }
      const sent = await taskApi.sendMessage(created.id, { content });
      setConversation(sent);
      setConversations((current) => upsertSummary(current, sent));
      await navigate(`/tasks/${created.id}`);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setPending(false);
    }
  };

  const deleteConversation = async (id: string) => {
    setPending(true);
    setError(null);
    try {
      await taskApi.delete(id);
      setConversations((current) => current.filter((item) => item.id !== id));
      if (conversation?.id === id) await navigate('/tasks');
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setPending(false);
    }
  };

  return (
    <div
      className="grid h-full min-h-0 min-w-0 grid-cols-[var(--tasks-sidebar-width)_0_minmax(0,1fr)] bg-background text-sm"
      style={
        {
          '--tasks-sidebar-width': `${sidebarWidth}px`,
        } as CSSProperties
      }
    >
      <TasksSidebar
        conversations={conversations}
        onCreate={createConversation}
        onDelete={(id) => void deleteConversation(id)}
        onSelect={(id) => void navigate(`/tasks/${id}`)}
        pending={pending}
        selectedId={conversation?.id ?? null}
      />
      <TasksSidebarResizeHandle
        onWidthChange={setSidebarWidth}
        width={sidebarWidth}
      />
      <AgentConversationView
        conversation={conversation}
        error={error}
        onConfigChange={(configId, value) =>
          conversation &&
          runAction(() =>
            taskApi.setConfig(conversation.id, { configId, value }),
          )
        }
        onModeChange={(modeId) =>
          conversation &&
          runAction(() => taskApi.setMode(conversation.id, { modeId }))
        }
        onDraftModelChange={(id, nextModelId) => {
          setProviderId(id);
          setModelId(nextModelId);
          persistTaskDraft(projectId, id, nextModelId);
        }}
        onProjectChange={(id) => {
          setProjectId(id);
          persistTaskDraft(id, providerId, modelId);
        }}
        onCancel={() =>
          conversation && runAction(() => taskApi.cancel(conversation.id))
        }
        onRespondPermission={(requestId, optionId) =>
          conversation &&
          runAction(() =>
            taskApi.respondPermission(conversation.id, requestId, {
              optionId,
            }),
          )
        }
        onSend={(content) => void sendMessage(content)}
        pending={pending}
        modelId={modelId}
        projectId={projectId}
        projects={initialData.projects}
        providerId={providerId}
        providers={initialData.providers.providers}
      />
    </div>
  );
}

function latestConversation(
  current: AgentConversation | null,
  next: AgentConversation,
): AgentConversation | null {
  if (current?.id !== next.id) return current;
  return next.updatedAt >= current.updatedAt ? next : current;
}

function upsertSummary(
  current: AgentConversationSummary[],
  conversation: AgentConversation,
): AgentConversationSummary[] {
  const summary: AgentConversationSummary = {
    activityAt: conversation.activityAt,
    autoTitle: conversation.autoTitle,
    createdAt: conversation.createdAt,
    id: conversation.id,
    projectId: conversation.projectId,
    projectName: conversation.projectName,
    providerId: conversation.providerId,
    status: conversation.status,
    updatedAt: conversation.updatedAt,
  };
  return [summary, ...current.filter((item) => item.id !== summary.id)].sort(
    (left, right) => right.activityAt.localeCompare(left.activityAt),
  );
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
