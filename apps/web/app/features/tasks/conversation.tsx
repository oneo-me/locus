import {
  agentConversationDisplayTitle,
  type AgentConversation,
  AgentPermissionEntry,
  AgentTranscriptEntry,
} from '@locus/agent/contracts/conversation';
import type { MarkdownDocument } from 'comark';
import { parseMarkdown } from 'comark';
import {
  BrainIcon,
  CheckCircle2Icon,
  CircleIcon,
  CircleXIcon,
  EllipsisIcon,
  LoaderCircleIcon,
  PanelRightIcon,
  PencilIcon,
  ShieldAlertIcon,
  WrenchIcon,
} from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { Alert, AlertDescription } from '@locus/ui/components/alert';
import { Button } from '@locus/ui/components/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@locus/ui/components/dropdown-menu';
import { Input } from '@locus/ui/components/input';
import { Separator } from '@locus/ui/components/separator';
import { cn } from '@locus/ui/utils';
import { MarkdownRenderer } from '~/components/markdown/renderer';
import { AgentComposer } from './composer';
import { NewTask } from './new-task';
import type { AgentProject } from '@locus/agent/contracts/conversation';
import type {
  AgentProviderId,
  AgentProviderProbe,
} from '@locus/agent/contracts/provider';

export function AgentConversationView({
  conversation,
  error,
  onCancel,
  onConfigChange,
  onDraftModelChange,
  onModeChange,
  onProjectChange,
  onRespondPermission,
  onRename,
  onSend,
  pending,
  modelId,
  projectId,
  projects,
  providerId,
  providers,
}: {
  conversation: AgentConversation | null;
  error: string | null;
  onCancel: () => void;
  onConfigChange: (configId: string, value: boolean | string) => void;
  onDraftModelChange: (providerId: AgentProviderId, modelId: string) => void;
  onModeChange: (modeId: string) => void;
  onProjectChange: (id: string) => void;
  onRespondPermission: (requestId: string, optionId: string) => void;
  onRename: (title: string) => void;
  onSend: (content: string) => void;
  pending: boolean;
  modelId: string | null;
  projectId: string | null;
  projects: AgentProject[];
  providerId: AgentProviderId | null;
  providers: AgentProviderProbe[];
}) {
  const end = useRef<HTMLDivElement>(null);
  const [renaming, setRenaming] = useState(false);
  const [title, setTitle] = useState('');
  useEffect(() => {
    end.current?.scrollIntoView({ block: 'end' });
  }, [conversation?.transcript]);

  return (
    <main className="flex min-h-0 min-w-0 flex-col bg-card" id="main-content">
      <header className="flex h-12 shrink-0 items-center gap-3 px-4">
        <div className="min-w-0 flex-1">
          {renaming && conversation ? (
            <form
              onSubmit={(event) => {
                event.preventDefault();
                if (!title.trim()) return;
                onRename(title);
                setRenaming(false);
              }}
            >
              <Input
                autoFocus
                className="h-8 max-w-80"
                onBlur={() => setRenaming(false)}
                onChange={(event) => setTitle(event.target.value)}
                value={title}
              />
            </form>
          ) : (
            <h2 className="truncate text-sm">
              {conversation
                ? agentConversationDisplayTitle(conversation)
                : '新任务'}
            </h2>
          )}
        </div>
        {conversation && (
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button aria-label="任务操作" size="icon-sm" variant="ghost">
                  <EllipsisIcon aria-hidden="true" />
                </Button>
              }
            />
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                onClick={() => {
                  setTitle(agentConversationDisplayTitle(conversation));
                  setRenaming(true);
                }}
              >
                <PencilIcon aria-hidden="true" />
                重命名
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        )}
        <Button
          aria-disabled="true"
          aria-label="打开右侧面板"
          size="icon-sm"
          tabIndex={-1}
          variant="ghost"
        >
          <PanelRightIcon aria-hidden="true" />
        </Button>
      </header>
      <Separator />
      <section aria-label="对话内容" className="flex min-h-0 flex-1 flex-col">
        <div className="flex min-h-0 flex-1 overflow-y-auto px-5 py-6">
          {conversation && conversation.transcript.length > 0 ? (
            <div className="mx-auto flex w-full max-w-[45rem] flex-col gap-5">
              {conversation.transcript.map((entry) => (
                <TranscriptEntry
                  entry={entry}
                  key={entry.id}
                  onRespondPermission={onRespondPermission}
                  pending={pending}
                />
              ))}
              {error && (
                <Alert variant="destructive">
                  <CircleXIcon aria-hidden="true" />
                  <AlertDescription>{error}</AlertDescription>
                </Alert>
              )}
              <div ref={end} />
            </div>
          ) : (
            <div className="m-auto flex w-full max-w-2xl flex-col gap-5">
              <NewTask
                onProjectChange={onProjectChange}
                pending={pending}
                projectId={projectId}
                projects={projects}
              />
              {error && (
                <Alert variant="destructive">
                  <CircleXIcon aria-hidden="true" />
                  <AlertDescription>{error}</AlertDescription>
                </Alert>
              )}
            </div>
          )}
        </div>
        <AgentComposer
          conversation={conversation}
          onCancel={onCancel}
          onConfigChange={onConfigChange}
          onDraftModelChange={onDraftModelChange}
          onModeChange={onModeChange}
          onSend={onSend}
          pending={pending}
          modelId={modelId}
          projectId={projectId}
          projects={projects}
          providerId={providerId}
          providers={providers}
        />
      </section>
    </main>
  );
}

function TranscriptEntry({
  entry,
  onRespondPermission,
  pending,
}: {
  entry: AgentTranscriptEntry;
  onRespondPermission: (requestId: string, optionId: string) => void;
  pending: boolean;
}) {
  if (entry.kind === 'message') {
    return entry.role === 'user' ? (
      <div className="ml-auto max-w-[85%] whitespace-pre-wrap rounded-2xl bg-muted px-3.5 py-2.5">
        {entry.content}
      </div>
    ) : (
      <AgentMarkdown content={entry.content} />
    );
  }
  if (entry.kind === 'thought') {
    return (
      <div className="flex gap-2 text-muted-foreground">
        {entry.streaming ? (
          <LoaderCircleIcon className="mt-0.5 size-4 shrink-0 motion-safe:animate-spin" />
        ) : (
          <BrainIcon className="mt-0.5 size-4 shrink-0" />
        )}
        <div className="whitespace-pre-wrap">{entry.content}</div>
      </div>
    );
  }
  if (entry.kind === 'tool') {
    return (
      <div className="rounded-lg border px-3 py-2.5">
        <div className="flex items-center gap-2">
          <ToolStatus status={entry.status} />
          <WrenchIcon className="size-3.5 text-muted-foreground" />
          <span>{entry.title}</span>
        </div>
        {entry.content && (
          <pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap text-xs text-muted-foreground">
            {entry.content}
          </pre>
        )}
      </div>
    );
  }
  if (entry.kind === 'plan') {
    return (
      <div className="rounded-lg border px-3 py-2.5">
        <ul className="space-y-2">
          {entry.entries.map((item, index) => (
            <li className="flex gap-2" key={`${index}:${item.content}`}>
              <ToolStatus status={item.status} />
              <span
                className={cn(
                  item.status === 'completed' && 'text-muted-foreground',
                )}
              >
                {item.content}
              </span>
            </li>
          ))}
        </ul>
      </div>
    );
  }
  if (entry.kind === 'permission') {
    return (
      <PermissionEntry
        entry={entry}
        onRespond={onRespondPermission}
        pending={pending}
      />
    );
  }
  return (
    <Alert variant="destructive">
      <CircleXIcon aria-hidden="true" />
      <AlertDescription>{entry.content}</AlertDescription>
    </Alert>
  );
}

function PermissionEntry({
  entry,
  onRespond,
  pending,
}: {
  entry: AgentPermissionEntry;
  onRespond: (requestId: string, optionId: string) => void;
  pending: boolean;
}) {
  return (
    <div className="rounded-lg border px-3 py-3">
      <div className="flex items-start gap-2">
        <ShieldAlertIcon className="mt-0.5 size-4 shrink-0 text-warning" />
        <span>{entry.title}</span>
      </div>
      <div className="mt-3 flex flex-wrap gap-2">
        {entry.options.map((option) => (
          <Button
            disabled={entry.status !== 'pending' || pending}
            key={option.id}
            onClick={() => onRespond(entry.requestId, option.id)}
            size="sm"
            variant={option.kind.includes('reject') ? 'outline' : 'default'}
          >
            {option.name}
          </Button>
        ))}
      </div>
    </div>
  );
}

function ToolStatus({ status }: { status: string }) {
  if (status === 'completed') {
    return <CheckCircle2Icon className="size-3.5 shrink-0 text-success" />;
  }
  if (status === 'failed') {
    return <CircleXIcon className="size-3.5 shrink-0 text-destructive" />;
  }
  if (status === 'in_progress') {
    return (
      <LoaderCircleIcon className="size-3.5 shrink-0 motion-safe:animate-spin" />
    );
  }
  return <CircleIcon className="size-3.5 shrink-0 text-muted-foreground" />;
}

function AgentMarkdown({ content }: { content: string }) {
  const [document, setDocument] = useState<MarkdownDocument | null>(null);
  useEffect(() => {
    let current = true;
    void parseMarkdown(content, { linkify: true }).then((next) => {
      if (current) setDocument(next);
    });
    return () => {
      current = false;
    };
  }, [content]);
  return document ? (
    <MarkdownRenderer className="space-y-4" value={document} />
  ) : (
    <div className="whitespace-pre-wrap">{content}</div>
  );
}
