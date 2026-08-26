import {
  ChevronDownIcon,
  CircleAlertIcon,
  FolderOpenIcon,
  GitBranchIcon,
  LoaderCircleIcon,
  SquarePenIcon,
  XCircleIcon,
} from 'lucide-react';
import { Button } from '@locus/ui/components/button';
import { cn } from '@locus/ui/utils';

type ConversationStatus = 'idle' | 'working' | 'waiting' | 'failed';

interface ConversationPreview {
  activity: string;
  branch: string;
  id: string;
  status: ConversationStatus;
  title: string;
}

const CONVERSATIONS: ConversationPreview[] = [
  {
    activity: '正在工作',
    branch: 'main',
    id: 'agents-shell',
    status: 'working',
    title: '实现 Agent 工作区外壳',
  },
  {
    activity: '刚刚',
    branch: 'main',
    id: 'product-notes',
    status: 'idle',
    title: '整理产品定义',
  },
  {
    activity: '等待输入',
    branch: 'main',
    id: 'recent-changes',
    status: 'waiting',
    title: '检查项目文档',
  },
];

export function AgentsSidebar() {
  return (
    <aside className="flex min-h-0 flex-col bg-sidebar text-sidebar-foreground">
      <header className="flex h-12 shrink-0 items-center justify-between px-3">
        <h1 className="text-sm">对话</h1>
      </header>

      <div className="px-2.5 pb-2">
        <Button
          aria-disabled="true"
          className="w-full justify-start"
          size="nav"
          tabIndex={-1}
          variant="ghost"
        >
          <SquarePenIcon aria-hidden="true" data-icon="inline-start" />
          新建对话
        </Button>
      </div>

      <nav
        aria-label="对话列表"
        className="min-h-0 flex-1 overflow-y-auto px-2.5"
      >
        <div className="flex h-7 items-center gap-1.5 px-2 text-xs text-muted-foreground">
          <FolderOpenIcon aria-hidden="true" className="size-3.5" />
          <span className="min-w-0 flex-1 truncate">Locus</span>
          <ChevronDownIcon aria-hidden="true" className="size-3.5" />
        </div>
        <ul className="flex flex-col gap-0.5">
          {CONVERSATIONS.map((conversation, index) => (
            <li key={conversation.id}>
              <div
                aria-current={index === 0 ? 'page' : undefined}
                className={cn(
                  'flex min-h-[3.25rem] min-w-0 flex-col gap-1 rounded-lg px-2 py-1.5',
                  index === 0 &&
                    'bg-sidebar-accent text-sidebar-accent-foreground',
                )}
              >
                <span className="flex min-w-0 items-center gap-1.5 leading-5">
                  <span className="min-w-0 flex-1 truncate">
                    {conversation.title}
                  </span>
                  <ConversationStatusIcon status={conversation.status} />
                </span>
                <span className="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
                  <GitBranchIcon
                    aria-hidden="true"
                    className="size-3 shrink-0"
                  />
                  <span className="min-w-0 flex-1 truncate">
                    {conversation.branch}
                  </span>
                  <span className="shrink-0">{conversation.activity}</span>
                </span>
              </div>
            </li>
          ))}
        </ul>
      </nav>
    </aside>
  );
}

function ConversationStatusIcon({ status }: { status: ConversationStatus }) {
  if (status === 'idle') return null;
  if (status === 'working') {
    return (
      <LoaderCircleIcon
        aria-label="正在工作"
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
