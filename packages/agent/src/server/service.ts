import { randomUUID } from 'node:crypto';
import type {
  SessionNotification,
  ToolCallContent,
} from '@agentclientprotocol/sdk';
import type {
  AgentConversation,
  AgentConversationEvent,
  AgentConversationList,
  AgentProject,
  AgentPermissionEntry,
  AgentPlanTranscriptEntry,
  AgentThoughtEntry,
  AgentToolEntry,
} from '../contracts/conversation.js';
import {
  agentConversationCanDelete,
  agentConversationTitleFromPrompt,
} from '../contracts/conversation.js';
import {
  TASK_AGENT_PROVIDER_IDS,
  type AgentProviderId,
  type TaskAgentProviderId,
  isTaskAgentProviderId,
} from '../contracts/provider.js';
import { AcpRuntime } from './acp-runtime.js';
import type {
  AgentRuntime,
  AgentRuntimePermissionRequest,
} from './agent-runtime.js';
import { AgentConversationStore } from './conversation-store.js';
import {
  detectAgentProviders,
  findAgentProviderExecutable,
} from './provider-detection.js';
import { normalizeSessionConfig } from './session-config.js';
import { providerPromptContent } from './prompt-content.js';
import { readGrokSessionTitle } from './grok-title.js';
import { NativeTitleRefresh } from './native-title-refresh.js';
import { discoverAgentProviderModels } from './provider-models.js';
import { OpenCodeRuntime } from './opencode-runtime.js';

type AgentEventListener = (event: AgentConversationEvent) => void;

export class AgentService {
  readonly #listeners = new Set<AgentEventListener>();
  readonly #runtimes = new Map<string, AgentRuntime>();
  readonly #store: AgentConversationStore;
  readonly #titleRefreshes = new Map<string, NativeTitleRefresh>();

  constructor(dataDirectory: string) {
    this.#store = new AgentConversationStore(dataDirectory);
  }

  listConversations(): AgentConversationList {
    return { conversations: this.#store.list() };
  }

  getConversation(id: string): AgentConversation {
    const conversation = this.#store.get(id);
    if (!conversation) throw new Error('对话不存在。');
    return conversation;
  }

  async connect(id: string): Promise<AgentConversation> {
    const conversation = this.getConversation(id);
    await this.#ensureRuntime(conversation);
    return conversation;
  }

  createConversation(
    project: AgentProject,
    requestedProviderId?: AgentProviderId,
  ): AgentConversation {
    const providerId = this.#resolveProvider(requestedProviderId);
    const now = new Date().toISOString();
    const conversation: AgentConversation = {
      activityAt: now,
      autoTitle: null,
      availableCommands: [],
      configOptions: [],
      contextTokens: null,
      contextWindow: null,
      createdAt: now,
      cwd: project.sourcePath,
      id: randomUUID(),
      modes: null,
      projectId: project.id,
      projectName: project.name,
      providerSessionId: null,
      providerId,
      status: 'idle',
      transcript: [],
      updatedAt: now,
    };
    this.#save(conversation);
    return conversation;
  }

  async sendMessage(
    id: string,
    rawContent: string,
  ): Promise<AgentConversation> {
    const conversation = this.getConversation(id);
    const content = rawContent.trim();
    if (!content) throw new Error('消息不能为空。');
    if (content.length > 100_000) throw new Error('消息内容过长。');
    if (
      conversation.status === 'working' ||
      conversation.status === 'connecting'
    ) {
      throw new Error('当前对话仍在工作。');
    }
    const runtime = await this.#ensureRuntime(conversation);
    const now = new Date().toISOString();
    conversation.transcript.push({
      content,
      createdAt: now,
      id: randomUUID(),
      kind: 'message',
      role: 'user',
      streaming: false,
    });
    if (
      conversation.transcript.filter((entry) => entry.kind === 'message')
        .length === 1 &&
      conversation.autoTitle === null
    ) {
      conversation.autoTitle = agentConversationTitleFromPrompt(content);
    }
    conversation.status = 'working';
    conversation.activityAt = now;
    conversation.updatedAt = now;
    this.#save(conversation);
    const transcriptLength = conversation.transcript.length;
    void runtime
      .prompt(providerPromptContent(content, conversation.availableCommands))
      .then(
        (stopReason) =>
          this.#finishTurn(conversation, stopReason, transcriptLength),
        (error) => this.#failConversation(conversation, error),
      );
    return conversation;
  }

  async cancel(id: string): Promise<AgentConversation> {
    const conversation = this.getConversation(id);
    await this.#runtimes.get(id)?.cancel();
    this.#settleStreaming(conversation);
    conversation.status = 'idle';
    const now = new Date().toISOString();
    conversation.activityAt = now;
    conversation.updatedAt = now;
    this.#save(conversation);
    return conversation;
  }

  respondPermission(
    conversationId: string,
    requestId: string,
    optionId: string,
  ): AgentConversation {
    const conversation = this.getConversation(conversationId);
    const permission = conversation.transcript.find(
      (entry): entry is AgentPermissionEntry =>
        entry.kind === 'permission' && entry.requestId === requestId,
    );
    if (!permission || permission.status !== 'pending') {
      throw new Error('权限请求不存在或已经处理。');
    }
    if (!permission.options.some((option) => option.id === optionId)) {
      throw new Error('权限选项无效。');
    }
    const runtime = this.#runtimes.get(conversationId);
    if (!runtime?.respondPermission(requestId, optionId)) {
      throw new Error('权限请求已经失效。');
    }
    permission.status = 'selected';
    permission.selectedOptionId = optionId;
    conversation.status = 'working';
    conversation.updatedAt = new Date().toISOString();
    this.#save(conversation);
    return conversation;
  }

  async setMode(id: string, modeId: string): Promise<AgentConversation> {
    const conversation = this.getConversation(id);
    if (!conversation.modes?.available.some((mode) => mode.id === modeId)) {
      throw new Error('会话模式无效。');
    }
    const runtime = await this.#ensureRuntime(conversation);
    await runtime.setMode(modeId);
    conversation.modes.currentId = modeId;
    conversation.updatedAt = new Date().toISOString();
    this.#save(conversation);
    return conversation;
  }

  async setConfig(
    id: string,
    configId: string,
    value: boolean | string,
  ): Promise<AgentConversation> {
    const conversation = this.getConversation(id);
    const option = conversation.configOptions.find(
      (candidate) => candidate.id === configId,
    );
    if (!option || typeof option.currentValue !== typeof value) {
      throw new Error('会话配置无效。');
    }
    if (
      option.type === 'select' &&
      !option.options.some((candidate) => candidate.value === value)
    ) {
      throw new Error('会话配置值无效。');
    }
    const runtime = await this.#ensureRuntime(conversation);
    conversation.configOptions =
      option.category === 'model' && typeof value === 'string'
        ? await runtime.setModel(configId, value, conversation.configOptions)
        : await runtime.setConfig(configId, value);
    conversation.updatedAt = new Date().toISOString();
    this.#save(conversation);
    return conversation;
  }

  deleteConversation(id: string): void {
    const conversation = this.getConversation(id);
    if (!agentConversationCanDelete(conversation.status)) {
      throw new Error('工作中的对话不能删除。');
    }
    this.#runtimes.get(id)?.close();
    this.#runtimes.delete(id);
    this.#titleRefreshes.get(id)?.close();
    this.#titleRefreshes.delete(id);
    if (!this.#store.delete(id)) throw new Error('对话不存在。');
  }

  subscribe(listener: AgentEventListener): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  close(): void {
    for (const runtime of this.#runtimes.values()) runtime.close();
    this.#runtimes.clear();
    for (const refresh of this.#titleRefreshes.values()) refresh.close();
    this.#titleRefreshes.clear();
  }

  #startGrokTitleRefresh(conversation: AgentConversation): void {
    if (!conversation.providerSessionId) return;
    this.#titleRefresh(conversation).start(
      [0, 250, 750, 1_500, 3_000, 5_000, 7_500, 10_000],
      () => readGrokSessionTitle(conversation.providerSessionId ?? ''),
      (title) => this.#applyProviderTitle(conversation, title),
    );
  }

  #titleRefresh(conversation: AgentConversation): NativeTitleRefresh {
    const existing = this.#titleRefreshes.get(conversation.id);
    if (existing) return existing;
    const refresh = new NativeTitleRefresh();
    this.#titleRefreshes.set(conversation.id, refresh);
    return refresh;
  }

  #applyProviderTitle(conversation: AgentConversation, title: string): void {
    if (
      this.#store.get(conversation.id) !== conversation ||
      conversation.autoTitle === title
    ) {
      return;
    }
    conversation.autoTitle = title;
    conversation.updatedAt = new Date().toISOString();
    this.#save(conversation);
  }

  async #ensureRuntime(conversation: AgentConversation): Promise<AgentRuntime> {
    const existing = this.#runtimes.get(conversation.id);
    if (existing) return existing;
    if (!isTaskAgentProviderId(conversation.providerId)) {
      throw new Error('该服务商当前不支持任务会话。');
    }
    const binary = findAgentProviderExecutable(conversation.providerId);
    if (!binary) throw new Error('服务商 CLI 不可用。');
    conversation.status = 'connecting';
    this.#save(conversation);
    const persistedModelOption = conversation.configOptions.find(
      (option) => option.type === 'select' && option.category === 'model',
    );
    const selectedModel =
      persistedModelOption?.type === 'select'
        ? persistedModelOption.currentValue
        : undefined;
    let runtimeRef: AgentRuntime | null = null;
    const handlers = {
      onConnected: (
        sessionId: string,
        modes: AgentConversation['modes'],
        configOptions: AgentConversation['configOptions'],
      ) => {
        conversation.providerSessionId = sessionId;
        conversation.configOptions = configOptions;
        conversation.modes = modes;
        conversation.status = 'idle' as const;
        conversation.updatedAt = new Date().toISOString();
        this.#save(conversation);
      },
      onError: (message: string) =>
        this.#failConversation(conversation, message),
      onExit: () => {
        if (runtimeRef && this.#runtimes.get(conversation.id) === runtimeRef) {
          this.#runtimes.delete(conversation.id);
        }
      },
      onPermission: (permission: AgentRuntimePermissionRequest) =>
        this.#handlePermission(conversation, permission),
    };
    const runtime = await (
      conversation.providerId === 'opencode'
        ? discoverAgentProviderModels('opencode', binary).then((models) =>
            OpenCodeRuntime.start({
              binary,
              cwd: conversation.cwd,
              handlers: {
                onConnected: (sessionId, configOptions) =>
                  handlers.onConnected(sessionId, null, configOptions),
                onError: handlers.onError,
                onExit: handlers.onExit,
                onIdle: () => {
                  this.#settleStreaming(conversation);
                  conversation.status = 'idle';
                  const now = new Date().toISOString();
                  conversation.activityAt = now;
                  conversation.updatedAt = now;
                  this.#save(conversation);
                },
                onPermission: handlers.onPermission,
                onText: (kind, text) =>
                  this.#handleText(conversation, kind, text),
                onTitle: (title) =>
                  this.#applyProviderTitle(conversation, title),
                onTool: (tool) => this.#handleTool(conversation, tool),
                onUsage: (tokens, window) => {
                  if (tokens !== null) conversation.contextTokens = tokens;
                  if (window !== null) conversation.contextWindow = window;
                  conversation.updatedAt = new Date().toISOString();
                  this.#save(conversation);
                },
              },
              models,
              selectedModel,
              sessionId: conversation.providerSessionId,
            }),
          )
        : AcpRuntime.start({
            binary,
            cwd: conversation.cwd,
            handlers: {
              ...handlers,
              onUpdate: (notification) =>
                this.#handleUpdate(conversation, notification),
            },
            providerId: conversation.providerId,
            sessionId: conversation.providerSessionId,
          })
    ).catch((error) => {
      if (conversation.status === 'connecting') {
        this.#failConversation(conversation, error);
      }
      throw error;
    });
    runtimeRef = runtime;
    if (selectedModel) {
      const modelOption = conversation.configOptions.find(
        (option) =>
          option.type === 'select' &&
          option.category === 'model' &&
          option.options.some((candidate) => candidate.value === selectedModel),
      );
      if (modelOption) {
        try {
          conversation.configOptions = await runtime.setModel(
            modelOption.id,
            selectedModel,
            conversation.configOptions,
          );
          conversation.updatedAt = new Date().toISOString();
          this.#save(conversation);
        } catch (error) {
          runtime.close();
          throw error;
        }
      }
    }
    this.#runtimes.set(conversation.id, runtime);
    return runtime;
  }

  #handleUpdate(
    conversation: AgentConversation,
    notification: SessionNotification,
  ): void {
    if (
      conversation.providerSessionId &&
      notification.sessionId !== conversation.providerSessionId
    ) {
      return;
    }
    const update = notification.update;
    const now = new Date().toISOString();
    if (
      (update.sessionUpdate === 'agent_message_chunk' ||
        update.sessionUpdate === 'agent_thought_chunk') &&
      update.content.type === 'text'
    ) {
      const kind =
        update.sessionUpdate === 'agent_message_chunk' ? 'message' : 'thought';
      const existing = [...conversation.transcript]
        .reverse()
        .find(
          (entry) =>
            entry.kind === kind &&
            (entry.kind === 'message' || entry.kind === 'thought') &&
            entry.streaming,
        );
      if (existing) {
        if (existing.kind === 'message' || existing.kind === 'thought') {
          existing.content += update.content.text;
          existing.streaming = true;
        }
      } else if (kind === 'message') {
        conversation.transcript.push({
          content: update.content.text,
          createdAt: now,
          id: randomUUID(),
          kind,
          role: 'assistant',
          streaming: true,
        });
      } else {
        conversation.transcript.push({
          content: update.content.text,
          createdAt: now,
          id: randomUUID(),
          kind,
          streaming: true,
        } satisfies AgentThoughtEntry);
      }
    } else if (update.sessionUpdate === 'tool_call') {
      const tool: AgentToolEntry = {
        content: toolContentText(update.content),
        createdAt: now,
        id: `tool:${update.toolCallId}`,
        kind: 'tool',
        status: update.status ?? 'pending',
        title: update.title,
        toolKind: update.kind ?? null,
      };
      const index = conversation.transcript.findIndex(
        (entry) => entry.id === tool.id,
      );
      if (index === -1) conversation.transcript.push(tool);
      else conversation.transcript[index] = tool;
    } else if (update.sessionUpdate === 'tool_call_update') {
      const tool = conversation.transcript.find(
        (entry): entry is AgentToolEntry =>
          entry.kind === 'tool' && entry.id === `tool:${update.toolCallId}`,
      );
      if (tool) {
        if (update.title) tool.title = update.title;
        if (update.status) tool.status = update.status;
        if (update.kind) tool.toolKind = update.kind;
        if (update.content) tool.content = toolContentText(update.content);
      } else {
        conversation.transcript.push({
          content: toolContentText(update.content),
          createdAt: now,
          id: `tool:${update.toolCallId}`,
          kind: 'tool',
          status: update.status ?? 'pending',
          title: update.title ?? '工具调用',
          toolKind: update.kind ?? null,
        });
      }
    } else if (update.sessionUpdate === 'plan') {
      const plan: AgentPlanTranscriptEntry = {
        createdAt: now,
        entries: update.entries.map(({ content, priority, status }) => ({
          content,
          priority,
          status,
        })),
        id: 'plan:current',
        kind: 'plan',
      };
      const index = conversation.transcript.findIndex(
        (entry) => entry.id === plan.id,
      );
      if (index === -1) conversation.transcript.push(plan);
      else conversation.transcript[index] = plan;
    } else if (
      update.sessionUpdate === 'plan_update' &&
      update.plan.type === 'items'
    ) {
      const plan: AgentPlanTranscriptEntry = {
        createdAt: now,
        entries: update.plan.entries.map(({ content, priority, status }) => ({
          content,
          priority,
          status,
        })),
        id: `plan:${update.plan.planId}`,
        kind: 'plan',
      };
      const index = conversation.transcript.findIndex(
        (entry) => entry.id === plan.id,
      );
      if (index === -1) conversation.transcript.push(plan);
      else conversation.transcript[index] = plan;
    } else if (update.sessionUpdate === 'plan_removed') {
      const index = conversation.transcript.findIndex(
        (entry) => entry.id === `plan:${update.planId}`,
      );
      if (index !== -1) conversation.transcript.splice(index, 1);
    } else if (update.sessionUpdate === 'current_mode_update') {
      if (conversation.modes) {
        conversation.modes.currentId = update.currentModeId;
      }
    } else if (update.sessionUpdate === 'config_option_update') {
      conversation.configOptions = normalizeSessionConfig(update.configOptions);
    } else if (update.sessionUpdate === 'available_commands_update') {
      conversation.availableCommands = update.availableCommands.map(
        ({ description, name }) => ({ description, name }),
      );
    } else if (update.sessionUpdate === 'session_info_update') {
      if (update.title !== undefined) {
        const title = normalizeProviderTitle(
          conversation.providerId,
          update.title,
        );
        if (title !== undefined) {
          conversation.autoTitle = title;
          if (title) this.#titleRefreshes.get(conversation.id)?.resolve();
        }
      }
    } else if (update.sessionUpdate === 'usage_update') {
      conversation.contextTokens = update.used;
      conversation.contextWindow = update.size;
    }
    conversation.updatedAt = now;
    this.#save(conversation);
  }

  #handleText(
    conversation: AgentConversation,
    kind: 'message' | 'thought',
    text: string,
  ): void {
    const existing = [...conversation.transcript]
      .reverse()
      .find(
        (entry) =>
          entry.kind === kind &&
          (entry.kind === 'message' || entry.kind === 'thought') &&
          entry.streaming,
      );
    if (existing?.kind === 'message' || existing?.kind === 'thought') {
      existing.content += text;
    } else if (kind === 'message') {
      conversation.transcript.push({
        content: text,
        createdAt: new Date().toISOString(),
        id: randomUUID(),
        kind,
        role: 'assistant',
        streaming: true,
      });
    } else {
      conversation.transcript.push({
        content: text,
        createdAt: new Date().toISOString(),
        id: randomUUID(),
        kind,
        streaming: true,
      });
    }
    conversation.updatedAt = new Date().toISOString();
    this.#save(conversation);
  }

  #handleTool(
    conversation: AgentConversation,
    tool: Omit<AgentToolEntry, 'createdAt' | 'kind'>,
  ): void {
    const entry: AgentToolEntry = {
      ...tool,
      createdAt: new Date().toISOString(),
      kind: 'tool',
    };
    const index = conversation.transcript.findIndex(
      (candidate) => candidate.id === entry.id,
    );
    if (index === -1) conversation.transcript.push(entry);
    else conversation.transcript[index] = entry;
    conversation.updatedAt = new Date().toISOString();
    this.#save(conversation);
  }

  #handlePermission(
    conversation: AgentConversation,
    request: AgentRuntimePermissionRequest,
  ): void {
    conversation.transcript.push({
      createdAt: new Date().toISOString(),
      id: `permission:${request.requestId}`,
      kind: 'permission',
      options: request.options,
      requestId: request.requestId,
      selectedOptionId: null,
      status: 'pending',
      title: request.title,
    });
    conversation.status = 'waiting';
    conversation.updatedAt = new Date().toISOString();
    this.#save(conversation);
  }

  #finishTurn(
    conversation: AgentConversation,
    stopReason: string,
    transcriptLength: number,
  ): void {
    if (stopReason !== 'end_turn' && stopReason !== 'cancelled') {
      this.#failConversation(
        conversation,
        stopReason === 'refusal'
          ? 'Agent 拒绝了这个任务。'
          : `Agent 已停止：${stopReason}。`,
      );
      return;
    }
    if (
      stopReason === 'end_turn' &&
      conversation.transcript.length === transcriptLength
    ) {
      this.#failConversation(conversation, 'Agent 未返回任何内容。');
      return;
    }
    this.#settleStreaming(conversation);
    conversation.status = 'idle';
    const now = new Date().toISOString();
    conversation.activityAt = now;
    conversation.updatedAt = now;
    this.#save(conversation);
    if (conversation.providerId === 'grok' && stopReason === 'end_turn') {
      this.#startGrokTitleRefresh(conversation);
    }
  }

  #failConversation(conversation: AgentConversation, error: unknown): void {
    const content = error instanceof Error ? error.message : String(error);
    if (!content) return;
    this.#settleStreaming(conversation);
    conversation.transcript.push({
      content,
      createdAt: new Date().toISOString(),
      id: randomUUID(),
      kind: 'error',
    });
    conversation.status = 'failed';
    const now = new Date().toISOString();
    conversation.activityAt = now;
    conversation.updatedAt = now;
    this.#save(conversation);
  }

  #settleStreaming(conversation: AgentConversation): void {
    for (const entry of conversation.transcript) {
      if (entry.kind === 'message' || entry.kind === 'thought') {
        entry.streaming = false;
      } else if (entry.kind === 'permission' && entry.status === 'pending') {
        entry.status = 'cancelled';
      }
    }
  }

  #save(conversation: AgentConversation): void {
    this.#store.save(conversation);
    const event: AgentConversationEvent = {
      conversation,
      type: 'conversation',
    };
    for (const listener of this.#listeners) listener(event);
  }

  #resolveProvider(requested?: AgentProviderId): TaskAgentProviderId {
    if (requested) {
      if (!isTaskAgentProviderId(requested)) {
        throw new Error('该服务商当前不支持任务会话。');
      }
      if (!findAgentProviderExecutable(requested)) {
        throw new Error('服务商 CLI 不可用。');
      }
      return requested;
    }
    const installed = new Set(
      detectAgentProviders()
        .providers.filter((provider) => provider.installed)
        .map((provider) => provider.id),
    );
    const providerId = TASK_AGENT_PROVIDER_IDS.find((id) => installed.has(id));
    if (!providerId) throw new Error('没有检测到支持任务会话的服务商。');
    return providerId;
  }
}

function normalizeProviderTitle(
  providerId: AgentProviderId,
  rawTitle: string | null,
): string | null | undefined {
  const title = rawTitle?.trim() ?? '';
  if (providerId === 'opencode' && title.startsWith('New session - ')) {
    return undefined;
  }
  return title || null;
}

function toolContentText(
  content: ToolCallContent[] | null | undefined,
): string | null {
  if (!content?.length) return null;
  const parts = content.flatMap((item) => {
    if (item.type === 'content' && item.content.type === 'text') {
      return [item.content.text];
    }
    if (item.type === 'diff') return [`${item.path}`];
    if (item.type === 'terminal') return ['终端输出'];
    return [];
  });
  return parts.length ? parts.join('\n') : null;
}
