import type {
  AgentConversation,
  AgentProject,
} from '@locus/agent/contracts/conversation';
import type {
  AgentProviderId,
  AgentProviderProbe,
} from '@locus/agent/contracts/provider';
import {
  ArrowUpIcon,
  BrainCircuitIcon,
  FolderIcon,
  GaugeIcon,
  LockKeyholeIcon,
  Settings2Icon,
  SquareIcon,
  WrenchIcon,
} from 'lucide-react';
import { type KeyboardEvent, useState } from 'react';
import { Button } from '@locus/ui/components/button';
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupTextarea,
} from '@locus/ui/components/input-group';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@locus/ui/components/select';
import { AgentModelPicker } from './model-picker';
import { userPath } from './user-path';

export function AgentComposer({
  conversation,
  onCancel,
  onConfigChange,
  onDraftModelChange,
  onDraftReasoningEffortChange,
  onModeChange,
  onSend,
  pending,
  modelId,
  reasoningEffort,
  projectId,
  projects,
  providerId,
  providers,
}: {
  conversation: AgentConversation | null;
  onCancel: () => void;
  onConfigChange: (configId: string, value: boolean | string) => void;
  onDraftModelChange: (providerId: AgentProviderId, modelId: string) => void;
  onDraftReasoningEffortChange: (reasoningEffort: string) => void;
  onModeChange: (modeId: string) => void;
  onSend: (content: string) => void;
  pending: boolean;
  modelId: string | null;
  reasoningEffort: string | null;
  projectId: string | null;
  projects: AgentProject[];
  providerId: AgentProviderId | null;
  providers: AgentProviderProbe[];
}) {
  const [content, setContent] = useState('');
  const working =
    conversation?.status === 'connecting' ||
    conversation?.status === 'working' ||
    conversation?.status === 'waiting';
  const currentProviderId = conversation?.providerId ?? providerId;
  const canSend = Boolean(
    content.trim() && projectId && currentProviderId && !pending && !working,
  );
  const projectPath =
    conversation?.cwd ??
    projects.find((project) => project.id === projectId)?.sourcePath;
  const draftModels = providers.find(
    (provider) => provider.id === providerId,
  )?.models;
  const draftModel = draftModels?.find((model) => model.id === modelId);
  const draftReasoningEffort =
    draftModel?.reasoningEfforts.find((option) => option.id === reasoningEffort)
      ?.id ??
    draftModel?.defaultReasoningEffort ??
    draftModel?.reasoningEfforts[0]?.id;
  const draftModelSources = providers
    .filter(
      (provider) =>
        provider.installed &&
        provider.supportsTasks &&
        provider.models.length > 0,
    )
    .map((provider) => ({
      id: provider.id,
      name: provider.name,
      options: provider.models.map((model) => ({
        name: model.name,
        value: model.id,
      })),
    }));
  const modelOption = conversation?.configOptions.find(
    (option) => option.type === 'select' && option.category === 'model',
  );
  const currentProvider = providers.find(
    (provider) => provider.id === conversation?.providerId,
  );
  const commandQuery =
    content.startsWith('/') && !content.includes(' ')
      ? content.slice(1).toLocaleLowerCase()
      : null;
  const commands = (conversation?.availableCommands ?? []).filter((command) =>
    command.name.toLocaleLowerCase().includes(commandQuery ?? ''),
  );
  const submit = () => {
    if (!canSend) return;
    const message = content.trim();
    setContent('');
    onSend(message);
  };
  const keyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (
      event.key !== 'Enter' ||
      event.shiftKey ||
      event.nativeEvent.isComposing
    ) {
      return;
    }
    event.preventDefault();
    submit();
  };

  return (
    <div className="w-full px-5 pb-2">
      <div className="mx-auto w-full max-w-[45rem]">
        {commandQuery !== null && commands.length > 0 && (
          <div className="mb-1 max-h-56 overflow-y-auto rounded-lg border bg-popover p-1 shadow-md">
            {commands.map((command) => (
              <button
                className="flex w-full items-start gap-3 rounded-md px-2.5 py-2 text-left hover:bg-accent"
                key={command.name}
                onClick={() => setContent(`/${command.name} `)}
                type="button"
              >
                <span className="shrink-0">/{command.name}</span>
                <span className="truncate text-muted-foreground">
                  {command.description}
                </span>
              </button>
            ))}
          </div>
        )}
        <InputGroup className="bg-card! focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/50">
          <InputGroupTextarea
            aria-label="给 Agent 发消息"
            className="min-h-12 max-h-48"
            onChange={(event) => setContent(event.target.value)}
            onKeyDown={keyDown}
            placeholder="给 Agent 发消息…"
            rows={2}
            value={content}
          />
          <InputGroupAddon align="block-end" className="gap-1">
            <div className="flex min-w-0 flex-1 items-center gap-0.5 overflow-x-auto text-foreground">
              {!conversation && providerId && draftModels?.length ? (
                <AgentModelPicker
                  disabled={false}
                  onChange={onDraftModelChange}
                  selectedModelId={modelId ?? draftModels[0].id}
                  selectedProviderId={providerId}
                  sources={draftModelSources}
                />
              ) : null}
              {!conversation &&
              draftReasoningEffort &&
              draftModel?.reasoningEfforts.length ? (
                <Select
                  items={draftModel.reasoningEfforts.map((option) => ({
                    label: option.name,
                    value: option.id,
                  }))}
                  onValueChange={(value) =>
                    value && onDraftReasoningEffortChange(value)
                  }
                  value={draftReasoningEffort}
                >
                  <SelectTrigger
                    aria-label="推理强度"
                    className="max-w-44"
                    size="sm"
                    variant="ghost"
                  >
                    <BrainCircuitIcon aria-hidden="true" />
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent align="start">
                    <SelectGroup>
                      {draftModel.reasoningEfforts.map((item) => (
                        <SelectItem key={item.id} value={item.id}>
                          {item.name}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
              ) : null}
              {conversation && modelOption?.type === 'select' ? (
                <AgentModelPicker
                  disabled={pending}
                  onChange={(_, value) => onConfigChange(modelOption.id, value)}
                  selectedModelId={modelOption.currentValue}
                  selectedProviderId={conversation.providerId}
                  sources={[
                    {
                      id: conversation.providerId,
                      name: currentProvider?.name ?? conversation.providerId,
                      options: modelOption.options,
                    },
                  ]}
                />
              ) : null}
              {conversation?.modes ? (
                <Select
                  disabled={pending}
                  onValueChange={(value) => value && onModeChange(value)}
                  value={conversation.modes.currentId}
                >
                  <SelectTrigger
                    aria-label="会话模式"
                    className="max-w-40"
                    size="sm"
                    variant="ghost"
                  >
                    <WrenchIcon aria-hidden="true" />
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent align="start">
                    <SelectGroup>
                      {conversation.modes.available.map((mode) => (
                        <SelectItem key={mode.id} value={mode.id}>
                          {mode.name}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
              ) : null}
              {conversation?.configOptions
                .filter(
                  (option) =>
                    option !== modelOption &&
                    (option.category !== 'mode' || !conversation.modes),
                )
                .map((option) =>
                  option.type === 'select' ? (
                    <Select
                      disabled={pending}
                      key={option.id}
                      onValueChange={(value) =>
                        value && onConfigChange(option.id, value)
                      }
                      value={option.currentValue}
                    >
                      <SelectTrigger
                        aria-label={option.name}
                        className="max-w-44"
                        size="sm"
                        variant="ghost"
                      >
                        <ConfigIcon category={option.category} />
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent align="start">
                        <SelectGroup>
                          {option.options.map((item) => (
                            <SelectItem key={item.value} value={item.value}>
                              {item.name}
                            </SelectItem>
                          ))}
                        </SelectGroup>
                      </SelectContent>
                    </Select>
                  ) : (
                    <Button
                      aria-label={option.name}
                      disabled={pending}
                      iconSize="sm"
                      key={option.id}
                      onClick={() =>
                        onConfigChange(option.id, !option.currentValue)
                      }
                      size="nav"
                      variant="ghost"
                    >
                      <Settings2Icon
                        aria-hidden="true"
                        data-icon="inline-start"
                      />
                      {option.name} · {option.currentValue ? '开' : '关'}
                    </Button>
                  ),
                )}
            </div>
            {working ? (
              <InputGroupButton
                aria-label="停止"
                className="rounded-full"
                disabled={pending}
                onClick={onCancel}
                size="icon-xs"
                variant="default"
              >
                <SquareIcon aria-hidden="true" />
              </InputGroupButton>
            ) : (
              <InputGroupButton
                aria-label="发送消息"
                className="rounded-full"
                disabled={!canSend}
                onClick={submit}
                size="icon-xs"
                variant="default"
              >
                <ArrowUpIcon aria-hidden="true" />
              </InputGroupButton>
            )}
          </InputGroupAddon>
        </InputGroup>
        <div className="flex h-8 min-w-0 items-center gap-3 px-2 text-xs text-muted-foreground">
          {projectPath && (
            <span
              className="flex min-w-0 items-center gap-1.5"
              title={userPath(projectPath)}
            >
              <FolderIcon className="size-3.5 shrink-0" />
              <span className="truncate">{userPath(projectPath)}</span>
            </span>
          )}
          <span className="flex-1" />
          {conversation &&
            conversation.contextTokens !== null &&
            conversation.contextWindow !== null && (
              <span className="flex items-center gap-1.5 px-2 text-xs">
                <GaugeIcon className="size-3.5" />
                {Math.round(
                  (conversation.contextTokens / conversation.contextWindow) *
                    100,
                )}
                %
              </span>
            )}
        </div>
      </div>
    </div>
  );
}

function ConfigIcon({ category }: { category: string | null }) {
  const Icon =
    category === 'thought_level'
      ? BrainCircuitIcon
      : category === 'mode'
        ? LockKeyholeIcon
        : Settings2Icon;
  return <Icon aria-hidden="true" />;
}
