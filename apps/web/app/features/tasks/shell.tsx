import type {
  AgentConversation,
  AgentConversationSummary,
  AgentProject,
  AgentWorkspaceEvent,
} from '@locus/agent/contracts/conversation';
import {
  type AgentProviderDetection,
  type AgentProviderId,
} from '@locus/agent/contracts/provider';
import { type CSSProperties, useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
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
  const [sidebarWidth, setSidebarWidth] = useState(272);
  const [conversations, setConversations] = useState(initialData.conversations);
  const [conversation, setConversation] = useState(initialData.conversation);
  const [projectId, setProjectId] = useState(
    initialData.conversation?.projectId ?? null,
  );
  const [providerId, setProviderId] = useState<AgentProviderId | null>(
    initialData.conversation?.providerId ?? null,
  );
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  useEffect(() => {
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
      setConversation((current) => (current?.id === next.id ? next : current));
    };
    source.onopen = () => setError(null);
    source.onerror = () => setError('任务实时连接已断开。');
    return () => source.close();
  }, []);

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
      await taskApi.connect(created.id);
      await taskApi.sendMessage(created.id, { content });
      await navigate(`/tasks/${created.id}`);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setPending(false);
    }
  };

  const configureDraft = async (
    nextProjectId: string | null,
    nextProviderId: AgentProviderId | null,
  ) => {
    if (
      conversation?.transcript.length === 0 &&
      conversation.projectId === nextProjectId &&
      conversation.providerId === nextProviderId
    ) {
      return;
    }
    setProjectId(nextProjectId);
    setProviderId(nextProviderId);
    if (!nextProjectId || !nextProviderId) return;

    setPending(true);
    setError(null);
    let created: AgentConversation | null = null;
    try {
      created = await taskApi.create({
        projectId: nextProjectId,
        providerId: nextProviderId,
      });
      const connected = await taskApi.connect(created.id);
      if (conversation && conversation.transcript.length === 0) {
        await taskApi.delete(conversation.id);
      }
      setConversation(connected);
      setConversations((current) =>
        upsertSummary(
          current.filter((item) => item.id !== conversation?.id),
          connected,
        ),
      );
      await navigate(`/tasks/${connected.id}`);
    } catch (cause) {
      if (created) {
        await taskApi.delete(created.id).catch(() => undefined);
      }
      setError(errorMessage(cause));
    } finally {
      setPending(false);
    }
  };

  const deleteConversation = async () => {
    if (!conversation) return;
    setPending(true);
    setError(null);
    try {
      await taskApi.delete(conversation.id);
      setConversations((current) =>
        current.filter((item) => item.id !== conversation.id),
      );
      await navigate('/tasks');
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
        onDelete={() => void deleteConversation()}
        onModeChange={(modeId) =>
          conversation &&
          runAction(() => taskApi.setMode(conversation.id, { modeId }))
        }
        onProjectChange={(id) => void configureDraft(id, providerId)}
        onProviderChange={(id) =>
          void configureDraft(
            projectId ?? initialData.projects[0]?.id ?? null,
            id,
          )
        }
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
        onRename={(title) =>
          conversation &&
          runAction(() => taskApi.rename(conversation.id, { title }))
        }
        onSend={(content) => void sendMessage(content)}
        pending={pending}
        projectId={projectId}
        projects={initialData.projects}
        providerId={providerId}
        providers={initialData.providers.providers}
      />
    </div>
  );
}

function upsertSummary(
  current: AgentConversationSummary[],
  conversation: AgentConversation,
): AgentConversationSummary[] {
  const summary: AgentConversationSummary = {
    createdAt: conversation.createdAt,
    id: conversation.id,
    projectId: conversation.projectId,
    projectName: conversation.projectName,
    providerId: conversation.providerId,
    status: conversation.status,
    title: conversation.title,
    updatedAt: conversation.updatedAt,
  };
  return [summary, ...current.filter((item) => item.id !== summary.id)].sort(
    (left, right) => right.updatedAt.localeCompare(left.updatedAt),
  );
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
