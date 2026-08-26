import type {
  AgentConversationStatus,
  AgentConversationSummary,
} from '@locus/agent/contracts/conversation';
import { AGENT_PROVIDERS } from '@locus/agent/contracts/provider';
import {
  ChevronDownIcon,
  CircleAlertIcon,
  FolderOpenIcon,
  LoaderCircleIcon,
  SquarePenIcon,
  XCircleIcon,
} from 'lucide-react';
import { Button } from '@locus/ui/components/button';
import { cn } from '@locus/ui/utils';
import { AgentProviderIcon } from '~/features/agent/provider-icon';

export function TasksSidebar({
  conversations,
  onCreate,
  onSelect,
  pending,
  selectedId,
}: {
  conversations: AgentConversationSummary[];
  onCreate: () => void;
  onSelect: (id: string) => void;
  pending: boolean;
  selectedId: string | null;
}) {
  const groups = groupConversations(conversations);
  return (
    <aside className="flex min-h-0 flex-col bg-sidebar text-sidebar-foreground">
      <div className="px-2.5 py-2">
        <Button
          className="w-full justify-start"
          disabled={pending}
          onClick={onCreate}
          size="nav"
          variant="ghost"
        >
          <SquarePenIcon aria-hidden="true" data-icon="inline-start" />
          新建任务
        </Button>
      </div>
      <nav
        aria-label="任务列表"
        className="min-h-0 flex-1 overflow-y-auto px-2.5"
      >
        {[...groups.entries()].map(([project, items]) => (
          <div className="pb-2" key={project}>
            <div className="flex h-7 items-center gap-1.5 px-2 text-xs text-muted-foreground">
              <FolderOpenIcon aria-hidden="true" className="size-3.5" />
              <span className="min-w-0 flex-1 truncate">{project}</span>
              <ChevronDownIcon aria-hidden="true" className="size-3.5" />
            </div>
            <ul className="flex flex-col gap-0.5">
              {items.map((conversation) => (
                <li key={conversation.id}>
                  <button
                    aria-current={
                      conversation.id === selectedId ? 'page' : undefined
                    }
                    className={cn(
                      'flex h-8 w-full min-w-0 cursor-default items-center gap-1.5 rounded-lg px-2 text-left',
                      conversation.id === selectedId &&
                        'bg-sidebar-accent text-sidebar-accent-foreground',
                    )}
                    onClick={() => onSelect(conversation.id)}
                    type="button"
                  >
                    <AgentProviderIcon
                      className="size-3.5 text-muted-foreground"
                      label={providerName(conversation.providerId)}
                      providerId={conversation.providerId}
                    />
                    <span className="min-w-0 flex-1 truncate">
                      {conversation.title}
                    </span>
                    <ConversationStatusIcon status={conversation.status} />
                    <span className="shrink-0 text-xs text-muted-foreground">
                      {activityLabel(conversation)}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </nav>
    </aside>
  );
}

function groupConversations(conversations: AgentConversationSummary[]) {
  const groups = new Map<string, AgentConversationSummary[]>();
  for (const conversation of conversations) {
    const items = groups.get(conversation.projectName) ?? [];
    items.push(conversation);
    groups.set(conversation.projectName, items);
  }
  return groups;
}

function ConversationStatusIcon({
  status,
}: {
  status: AgentConversationStatus;
}) {
  if (status === 'idle') return null;
  if (status === 'connecting' || status === 'working') {
    return (
      <LoaderCircleIcon
        aria-label={status === 'connecting' ? '正在连接' : '正在工作'}
        className="size-3 shrink-0 text-success motion-safe:animate-spin"
        role="img"
      />
    );
  }
  if (status === 'waiting') {
    return (
      <CircleAlertIcon
        aria-label="等待输入"
        className="size-3 shrink-0 text-warning"
        role="img"
      />
    );
  }
  return (
    <XCircleIcon
      aria-label="执行失败"
      className="size-3 shrink-0 text-destructive"
      role="img"
    />
  );
}

function providerName(providerId: string): string {
  return (
    AGENT_PROVIDERS.find((provider) => provider.id === providerId)?.name ??
    providerId
  );
}

function activityLabel(conversation: AgentConversationSummary): string {
  if (conversation.status === 'connecting') return '正在连接';
  if (conversation.status === 'working') return '正在工作';
  if (conversation.status === 'waiting') return '等待输入';
  if (conversation.status === 'failed') return '失败';
  return new Intl.DateTimeFormat('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(conversation.updatedAt));
}
