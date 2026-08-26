import {
  agentConversationCanDelete,
  agentConversationDisplayTitle,
  type AgentConversationStatus,
  type AgentConversationSummary,
} from '@locus/agent/contracts/conversation';
import {
  ChevronDownIcon,
  ChevronRightIcon,
  CircleAlertIcon,
  FolderIcon,
  LoaderCircleIcon,
  SquarePenIcon,
  XCircleIcon,
} from 'lucide-react';
import { useEffect, useState } from 'react';
import { Button } from '@locus/ui/components/button';
import { cn } from '@locus/ui/utils';
import {
  groupConversationsByDate,
  nextSidebarTimeUpdateDelay,
  relativeTimeLabel,
  type TaskDateGroup,
} from './sidebar-presentation';
import { TaskDeleteButton } from './delete-button';

const sidebarItemClassName =
  'w-full min-w-0 cursor-default justify-start rounded-lg px-2 text-left outline-none hover:bg-sidebar-accent focus-visible:border-transparent focus-visible:ring-1 focus-visible:ring-sidebar-ring has-data-[icon=inline-start]:pl-2';

export function TasksSidebar({
  conversations,
  onCreate,
  onDelete,
  onSelect,
  pending,
  selectedId,
}: {
  conversations: AgentConversationSummary[];
  onCreate: () => void;
  onDelete: (id: string) => void;
  onSelect: (id: string) => void;
  pending: boolean;
  selectedId: string | null;
}) {
  const [collapsedGroups, setCollapsedGroups] = useState<Set<TaskDateGroup>>(
    new Set(),
  );
  const [now, setNow] = useState(() => new Date());
  const groups = groupConversationsByDate(conversations, now);

  useEffect(() => {
    const timeout = window.setTimeout(
      () => setNow(new Date()),
      nextSidebarTimeUpdateDelay(conversations, now),
    );
    return () => window.clearTimeout(timeout);
  }, [conversations, now]);

  return (
    <aside className="flex min-h-0 flex-col bg-sidebar text-sidebar-foreground">
      <div className="px-2.5 py-2">
        <Button
          className={cn(sidebarItemClassName, 'h-8 gap-1.5')}
          disabled={pending}
          onClick={onCreate}
          size="nav"
          variant="nav"
        >
          <SquarePenIcon aria-hidden="true" data-icon="inline-start" />
          新建任务
        </Button>
      </div>
      <nav
        aria-label="任务列表"
        className="min-h-0 flex-1 overflow-y-auto px-2.5"
      >
        {groups.map((group) => {
          const collapsed = collapsedGroups.has(group.id);
          return (
            <section className="pb-2.5" key={group.id}>
              <button
                aria-expanded={!collapsed}
                className="flex h-7 items-center gap-1 px-2 text-xs font-medium text-muted-foreground outline-none hover:text-sidebar-foreground focus-visible:ring-1 focus-visible:ring-sidebar-ring"
                onClick={() =>
                  setCollapsedGroups((current) => {
                    const next = new Set(current);
                    if (next.has(group.id)) next.delete(group.id);
                    else next.add(group.id);
                    return next;
                  })
                }
                type="button"
              >
                {group.label}
                {collapsed ? (
                  <ChevronRightIcon aria-hidden="true" className="size-3" />
                ) : (
                  <ChevronDownIcon aria-hidden="true" className="size-3" />
                )}
              </button>
              {!collapsed && (
                <ul className="flex flex-col gap-px">
                  {group.conversations.map((conversation) => {
                    const canDelete = agentConversationCanDelete(
                      conversation.status,
                    );
                    return (
                      <li className="group relative" key={conversation.id}>
                        <Button
                          aria-current={
                            conversation.id === selectedId ? 'page' : undefined
                          }
                          className={cn(
                            sidebarItemClassName,
                            'h-13 flex-col items-stretch gap-1 py-1.5 whitespace-normal',
                            conversation.id === selectedId &&
                              'bg-sidebar-accent text-sidebar-accent-foreground',
                          )}
                          onClick={() => onSelect(conversation.id)}
                          size="nav"
                          type="button"
                          variant="nav"
                        >
                          <span
                            className={cn(
                              'flex w-full min-w-0 items-center gap-1.5 leading-5',
                              canDelete && 'pr-6',
                            )}
                          >
                            <span className="min-w-0 flex-1 truncate text-sm">
                              {agentConversationDisplayTitle(conversation)}
                            </span>
                            <span
                              className={cn(
                                canDelete &&
                                  'group-hover:invisible group-focus-within:invisible',
                              )}
                            >
                              <ConversationStatusIcon
                                status={conversation.status}
                              />
                            </span>
                          </span>
                          <span className="flex w-full min-w-0 items-center gap-1.5 text-xs leading-4 text-muted-foreground">
                            <FolderIcon
                              aria-hidden="true"
                              className="size-3 shrink-0"
                            />
                            <span className="min-w-0 flex-1 truncate">
                              {conversation.projectName}
                            </span>
                            <span className="shrink-0 text-muted-foreground/70">
                              {relativeTimeLabel(conversation.activityAt, now)}
                            </span>
                          </span>
                        </Button>
                        {canDelete && (
                          <TaskDeleteButton
                            className="pointer-events-none absolute top-1.5 right-2 opacity-0 group-hover:pointer-events-auto group-hover:opacity-100 group-focus-within:pointer-events-auto group-focus-within:opacity-100"
                            disabled={pending}
                            onDelete={() => onDelete(conversation.id)}
                          />
                        )}
                      </li>
                    );
                  })}
                </ul>
              )}
            </section>
          );
        })}
      </nav>
    </aside>
  );
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
