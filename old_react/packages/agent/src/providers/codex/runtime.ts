import { randomUUID } from 'node:crypto';
import type { AgentSessionConfig } from '../../contracts/conversation.js';
import type { AgentProviderModel } from '../../contracts/provider.js';
import type {
  AgentRuntime,
  NativeAgentRuntimeHandlers,
} from '../../server/agent-runtime.js';
import { JsonLineProcess } from '../../server/json-line-process.js';
import {
  MODEL_CONFIG_ID,
  REASONING_EFFORT_CONFIG_ID,
  modelConfigOptions,
} from '../../server/model-config.js';

interface CodexRuntimeOptions {
  binary: string;
  cwd: string;
  handlers: NativeAgentRuntimeHandlers;
  models: AgentProviderModel[];
  selectedModel?: string;
  selectedReasoningEffort?: string;
  sessionId: string | null;
}

interface Deferred<T> {
  promise: Promise<T>;
  reject: (reason?: unknown) => void;
  resolve: (value: T) => void;
}

export class CodexRuntime implements AgentRuntime {
  readonly #connected = deferred<void>();
  readonly #cwd: string;
  readonly #handlers: NativeAgentRuntimeHandlers;
  readonly #models: AgentProviderModel[];
  readonly #process: JsonLineProcess;
  readonly #requestedSessionId: string | null;
  #closed = false;
  #model: string | undefined;
  #reasoningEffort: string | undefined;
  #requestId = 10;
  #sessionId: string | null = null;
  #turn: Deferred<string> | null = null;
  #turnId: string | null = null;

  private constructor(options: CodexRuntimeOptions) {
    this.#cwd = options.cwd;
    this.#handlers = options.handlers;
    this.#models = options.models;
    this.#model = options.selectedModel ?? defaultModel(options.models);
    this.#reasoningEffort = options.selectedReasoningEffort;
    this.#reasoningEffort = this.#resolvedReasoningEffort();
    this.#requestedSessionId = options.sessionId;
    this.#process = new JsonLineProcess({
      args: ['app-server', '--stdio'],
      binary: options.binary,
      cwd: options.cwd,
      onExit: (message) => {
        const error = message ?? 'Codex 进程已退出。';
        this.#connected.reject(new Error(error));
        this.#turn?.reject(new Error(error));
        this.#turn = null;
        if (message) this.#handlers.onError(message);
        this.#handlers.onExit();
      },
      onMessage: (message) => this.#handle(message),
    });
    this.#initialize();
  }

  static async start(options: CodexRuntimeOptions): Promise<CodexRuntime> {
    const runtime = new CodexRuntime(options);
    await runtime.#connected.promise;
    return runtime;
  }

  async prompt(content: string): Promise<string> {
    if (!this.#sessionId) throw new Error('Codex thread 尚未连接。');
    if (this.#turn) throw new Error('Codex 仍在处理上一条消息。');
    const turn = deferred<string>();
    this.#turn = turn;
    const params: Record<string, unknown> = {
      approvalPolicy: 'untrusted',
      approvalsReviewer: 'user',
      input: [{ text: content, type: 'text' }],
      sandboxPolicy: { type: 'readOnly' },
      summary: 'auto',
      threadId: this.#sessionId,
    };
    if (this.#model) params.model = this.#model;
    if (this.#reasoningEffort) params.effort = this.#reasoningEffort;
    this.#send('turn/start', params);
    return turn.promise;
  }

  async cancel(): Promise<void> {
    if (!this.#sessionId || !this.#turnId) return;
    this.#send('turn/interrupt', {
      threadId: this.#sessionId,
      turnId: this.#turnId,
    });
  }

  async setMode(): Promise<void> {
    throw new Error('Codex runtime 不提供会话模式切换。');
  }

  async setConfig(
    configId: string,
    value: boolean | string,
  ): Promise<AgentSessionConfig[]> {
    if (typeof value !== 'string') {
      throw new Error('Codex 会话配置无效。');
    }
    if (configId === MODEL_CONFIG_ID) {
      return this.setModel(configId, value, this.#configOptions());
    }
    if (
      configId !== REASONING_EFFORT_CONFIG_ID ||
      !this.#currentModel()?.reasoningEfforts.some(
        (effort) => effort.id === value,
      )
    ) {
      throw new Error('Codex 推理强度无效。');
    }
    this.#reasoningEffort = value;
    return this.#configOptions();
  }

  async setModel(
    configId: string,
    modelId: string,
    _configOptions: AgentSessionConfig[],
  ): Promise<AgentSessionConfig[]> {
    if (
      configId !== MODEL_CONFIG_ID ||
      !this.#models.some((model) => model.id === modelId)
    ) {
      throw new Error('Codex 模型无效。');
    }
    this.#model = modelId;
    this.#reasoningEffort = this.#resolvedReasoningEffort();
    return this.#configOptions();
  }

  respondPermission(requestId: string, optionId: string | null): boolean {
    if (!optionId) return false;
    this.#process.send({
      id: rpcId(requestId),
      result: { decision: optionId },
    });
    return true;
  }

  close(): void {
    if (this.#closed) return;
    this.#closed = true;
    this.#turn?.reject(new Error('Codex 会话已关闭。'));
    this.#turn = null;
    this.#process.close();
  }

  #initialize(): void {
    this.#process.send({
      id: 0,
      method: 'initialize',
      params: {
        capabilities: { experimentalApi: true },
        clientInfo: { name: 'locus', title: 'Locus', version: '0.1.0' },
      },
    });
    this.#process.send({ method: 'initialized', params: {} });
    const params: Record<string, unknown> = {
      approvalPolicy: 'untrusted',
      approvalsReviewer: 'user',
      cwd: this.#cwd,
      sandbox: 'read-only',
    };
    if (this.#model) params.model = this.#model;
    if (!this.#requestedSessionId) params.serviceName = 'locus';
    this.#process.send({
      id: 1,
      method: this.#requestedSessionId ? 'thread/resume' : 'thread/start',
      params: this.#requestedSessionId
        ? { ...params, threadId: this.#requestedSessionId }
        : params,
    });
  }

  #send(method: string, params: Record<string, unknown>): void {
    this.#requestId += 1;
    this.#process.send({ id: this.#requestId, method, params });
  }

  #handle(value: Record<string, unknown>): void {
    if (value.method == null && value.id === 1) {
      const thread = record(record(value.result)?.thread);
      const sessionId = text(thread?.id);
      if (!sessionId) {
        const message =
          text(record(value.error)?.message) ?? 'Codex 未返回 thread ID。';
        this.#connected.reject(new Error(message));
        return;
      }
      this.#sessionId = sessionId;
      const title = text(thread?.name)?.trim();
      if (title) this.#handlers.onTitle(title);
      this.#handlers.onConnected(sessionId, this.#configOptions());
      this.#connected.resolve();
      return;
    }
    const method = text(value.method);
    const params = record(value.params) ?? {};
    if (!method) {
      const message = text(record(value.error)?.message);
      if (message) this.#handlers.onError(message);
      return;
    }
    if (method === 'turn/started') {
      this.#turnId = text(record(params.turn)?.id) ?? null;
    } else if (method === 'item/agentMessage/delta') {
      const delta = text(params.delta);
      if (delta) this.#handlers.onText('message', delta);
    } else if (
      method === 'item/reasoning/summaryTextDelta' ||
      method === 'item/reasoning/textDelta'
    ) {
      const delta = text(params.delta);
      if (delta) this.#handlers.onText('thought', delta);
    } else if (method === 'item/started' || method === 'item/completed') {
      const item = record(params.item);
      if (item) this.#handleItem(item, method === 'item/completed');
    } else if (method === 'turn/completed') {
      this.#turnId = null;
      const turnData = record(params.turn);
      const status = text(turnData?.status) ?? 'completed';
      const error = text(record(turnData?.error)?.message);
      const turn = this.#turn;
      this.#turn = null;
      if (!turn) this.#handlers.onIdle();
      else if (status === 'completed') turn.resolve('end_turn');
      else turn.reject(new Error(error ?? `Codex 回合已停止：${status}。`));
    } else if (method === 'thread/name/updated') {
      if (text(params.threadId) === this.#sessionId) {
        const title = text(params.threadName)?.trim();
        if (title) this.#handlers.onTitle(title);
      }
    } else if (method === 'thread/tokenUsage/updated') {
      const usage = record(params.tokenUsage);
      const tokens = number(record(usage?.last)?.totalTokens);
      const window = number(usage?.modelContextWindow);
      this.#handlers.onUsage(tokens, window);
    } else if (method === 'error') {
      const message = text(params.message);
      if (message) this.#handlers.onError(message);
    } else if (value.id != null && method.includes('requestApproval')) {
      const requestId = requestIdentifier(value.id);
      if (!requestId) return;
      const command = text(params.command) ?? text(params.reason) ?? '执行操作';
      this.#handlers.onPermission({
        options: [
          { id: 'accept', kind: 'allow_once', name: '允许一次' },
          {
            id: 'acceptForSession',
            kind: 'allow_always',
            name: '本次会话允许',
          },
          { id: 'decline', kind: 'reject_once', name: '拒绝' },
        ],
        requestId,
        title: command,
      });
    }
  }

  #handleItem(item: Record<string, unknown>, complete: boolean): void {
    const type = text(item.type);
    if (!type || type === 'agentMessage' || type === 'reasoning') return;
    const id = text(item.id) ?? randomUUID();
    const command = text(item.command);
    const title =
      command ??
      text(item.name) ??
      text(item.path) ??
      type.replace(/([a-z])([A-Z])/g, '$1 $2');
    const failed = text(item.status) === 'failed' || item.error != null;
    this.#handlers.onTool({
      content: jsonText(item.output ?? item.result ?? item.error),
      id: `tool:${id}`,
      status: failed ? 'failed' : complete ? 'completed' : 'in_progress',
      title,
      toolKind: type,
    });
  }

  #configOptions(): AgentSessionConfig[] {
    return modelConfigOptions(
      this.#models,
      this.#model,
      this.#resolvedReasoningEffort(),
    );
  }

  #currentModel(): AgentProviderModel | undefined {
    return this.#models.find((model) => model.id === this.#model);
  }

  #resolvedReasoningEffort(): string | undefined {
    const model = this.#currentModel();
    if (!model?.reasoningEfforts.length) return undefined;
    return model.reasoningEfforts.some(
      (effort) => effort.id === this.#reasoningEffort,
    )
      ? this.#reasoningEffort
      : (model.defaultReasoningEffort ?? model.reasoningEfforts[0]?.id);
  }
}

function defaultModel(models: AgentProviderModel[]): string | undefined {
  return models.find((model) => model.isDefault)?.id ?? models[0]?.id;
}

function rpcId(value: string): string | number {
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) ? parsed : value;
}

function jsonText(value: unknown): string | null {
  if (value == null) return null;
  return typeof value === 'string' ? value : JSON.stringify(value, null, 2);
}

function record(value: unknown): Record<string, unknown> | null {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function text(value: unknown): string | undefined {
  return typeof value === 'string' ? value : undefined;
}

function requestIdentifier(value: unknown): string | undefined {
  return typeof value === 'string' || typeof value === 'number'
    ? String(value)
    : undefined;
}

function number(value: unknown): number | null {
  return typeof value === 'number' && value > 0 ? value : null;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, reject, resolve };
}
