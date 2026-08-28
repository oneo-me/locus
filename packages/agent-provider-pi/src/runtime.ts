import { randomUUID } from 'node:crypto';
import type { AgentSessionConfig } from '@locus/agent/contracts/conversation';
import type { AgentProviderModel } from '@locus/agent/contracts/provider';
import type {
  AgentRuntime,
  NativeAgentRuntimeHandlers,
} from '@locus/agent/server/agent-runtime';
import { JsonLineProcess } from '@locus/agent/server/json-line-process';
import {
  MODEL_CONFIG_ID,
  REASONING_EFFORT_CONFIG_ID,
  modelConfigOptions,
} from '@locus/agent/server/model-config';

interface PiRuntimeOptions {
  binary: string;
  cwd: string;
  handlers: NativeAgentRuntimeHandlers;
  models: AgentProviderModel[];
  selectedModel?: string;
  selectedReasoningEffort?: string;
  sessionPath: string | null;
}

interface Deferred<T> {
  promise: Promise<T>;
  reject: (reason?: unknown) => void;
  resolve: (value: T) => void;
}

interface PendingRequest {
  deferred: Deferred<Record<string, unknown>>;
  timeout: NodeJS.Timeout;
}

export class PiRuntime implements AgentRuntime {
  readonly #handlers: NativeAgentRuntimeHandlers;
  readonly #models: AgentProviderModel[];
  readonly #pending = new Map<string, PendingRequest>();
  readonly #process: JsonLineProcess;
  #closed = false;
  #model: string | undefined;
  #reasoningEffort: string | undefined;
  #requestId = 0;
  #turn: Deferred<string> | null = null;

  private constructor(options: PiRuntimeOptions) {
    this.#handlers = options.handlers;
    this.#models = options.models;
    this.#model = options.selectedModel ?? defaultModel(options.models);
    this.#reasoningEffort = options.selectedReasoningEffort;
    this.#reasoningEffort = this.#resolvedReasoningEffort();
    this.#process = new JsonLineProcess({
      args: ['--mode', 'rpc', '--approve'],
      binary: options.binary,
      cwd: options.cwd,
      env: { PI_SKIP_VERSION_CHECK: '1' },
      onExit: (message) => {
        const error = new Error(message ?? 'Pi 进程已退出。');
        for (const pending of this.#pending.values()) {
          clearTimeout(pending.timeout);
          pending.deferred.reject(error);
        }
        this.#pending.clear();
        this.#turn?.reject(error);
        this.#turn = null;
        if (message) this.#handlers.onError(message);
        this.#handlers.onExit();
      },
      onMessage: (message) => this.#handle(message),
    });
  }

  static async start(options: PiRuntimeOptions): Promise<PiRuntime> {
    const runtime = new PiRuntime(options);
    await runtime.#request({ type: 'get_state' });
    if (options.sessionPath) {
      await runtime.#request({
        sessionPath: options.sessionPath,
        type: 'switch_session',
      });
    }
    if (runtime.#model && runtime.#model !== 'default') {
      await runtime.#selectModel(runtime.#model);
    }
    if (runtime.#reasoningEffort) {
      await runtime.#setReasoningEffort(runtime.#reasoningEffort);
    }
    const state = await runtime.#request({ type: 'get_state' });
    const data = record(state.data);
    const sessionId = text(data?.sessionId);
    if (!sessionId) {
      runtime.close();
      throw new Error('Pi 未返回 session ID。');
    }
    const sessionPath = text(data?.sessionFile) ?? null;
    const title = text(data?.sessionName)?.trim();
    if (title) options.handlers.onTitle(title);
    options.handlers.onConnected(
      sessionId,
      runtime.#configOptions(),
      sessionPath,
    );
    const model = record(data?.model);
    options.handlers.onUsage(null, positiveNumber(model?.contextWindow));
    return runtime;
  }

  async prompt(content: string): Promise<string> {
    if (this.#turn) throw new Error('Pi 仍在处理上一条消息。');
    const turn = deferred<string>();
    this.#turn = turn;
    void this.#request({ message: content, type: 'prompt' }).catch((error) => {
      if (this.#turn !== turn) return;
      this.#turn = null;
      turn.reject(error);
    });
    return turn.promise;
  }

  async cancel(): Promise<void> {
    await this.#request({ type: 'abort' });
  }

  async setMode(): Promise<void> {
    throw new Error('Pi 仅支持 Build + Full access。');
  }

  async setConfig(
    configId: string,
    value: boolean | string,
  ): Promise<AgentSessionConfig[]> {
    if (typeof value !== 'string') {
      throw new Error('Pi 会话配置无效。');
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
      throw new Error('Pi 推理强度无效。');
    }
    await this.#setReasoningEffort(value);
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
      throw new Error('Pi 模型无效。');
    }
    if (modelId !== 'default') await this.#selectModel(modelId);
    this.#model = modelId;
    const effort = this.#resolvedReasoningEffort();
    if (effort && effort !== this.#reasoningEffort) {
      await this.#setReasoningEffort(effort);
    }
    this.#reasoningEffort = effort;
    return this.#configOptions();
  }

  respondPermission(): boolean {
    return false;
  }

  close(): void {
    if (this.#closed) return;
    this.#closed = true;
    const error = new Error('Pi 会话已关闭。');
    for (const pending of this.#pending.values()) {
      clearTimeout(pending.timeout);
      pending.deferred.reject(error);
    }
    this.#pending.clear();
    this.#turn?.reject(error);
    this.#turn = null;
    this.#process.close();
  }

  async #selectModel(model: string): Promise<void> {
    const separator = model.indexOf('/');
    if (separator <= 0 || separator === model.length - 1) {
      throw new Error(`Pi 模型必须使用 provider/model 格式：${model}`);
    }
    await this.#request({
      modelId: model.slice(separator + 1),
      provider: model.slice(0, separator),
      type: 'set_model',
    });
  }

  async #setReasoningEffort(level: string): Promise<void> {
    await this.#request({ level, type: 'set_thinking_level' });
  }

  #request(value: Record<string, unknown>): Promise<Record<string, unknown>> {
    this.#requestId += 1;
    const id = `locus-${this.#requestId}`;
    const pending = deferred<Record<string, unknown>>();
    const timeout = setTimeout(() => {
      if (!this.#pending.delete(id)) return;
      pending.reject(new Error('Pi RPC 请求超时。'));
    }, 10_000);
    this.#pending.set(id, { deferred: pending, timeout });
    try {
      this.#process.send({ ...value, id });
    } catch (error) {
      this.#pending.delete(id);
      clearTimeout(timeout);
      pending.reject(error);
    }
    return pending.promise;
  }

  #handle(value: Record<string, unknown>): void {
    if (value.type === 'response') {
      const id = text(value.id);
      const pending = id ? this.#pending.get(id) : undefined;
      if (!id || !pending) return;
      this.#pending.delete(id);
      clearTimeout(pending.timeout);
      if (value.success === true) pending.deferred.resolve(value);
      else {
        pending.deferred.reject(
          new Error(text(value.error) ?? 'Pi RPC 请求失败。'),
        );
      }
      return;
    }
    if (value.type === 'session_info_changed') {
      const title = text(value.name)?.trim();
      if (title) this.#handlers.onTitle(title);
      return;
    }
    if (value.type === 'message_update') {
      const update = record(value.assistantMessageEvent);
      const delta = text(update?.delta);
      if (update?.type === 'text_delta' && delta) {
        this.#handlers.onText('message', delta);
      } else if (update?.type === 'thinking_delta' && delta) {
        this.#handlers.onText('thought', delta);
      } else if (update?.type === 'error') {
        const message = text(update.error) ?? 'Pi 返回错误。';
        this.#handlers.onError(message);
      }
      return;
    }
    if (value.type === 'message_end') {
      const message = record(value.message);
      if (message?.role === 'assistant') {
        this.#handlers.onUsage(piTokens(record(message.usage)), null);
      }
      return;
    }
    if (
      value.type === 'tool_execution_start' ||
      value.type === 'tool_execution_update' ||
      value.type === 'tool_execution_end'
    ) {
      const id = text(value.toolCallId) ?? randomUUID();
      const title = text(value.toolName) ?? '工具调用';
      const failed = value.isError === true;
      this.#handlers.onTool({
        content: jsonText(value.result ?? value.partialResult ?? value.args),
        id: `tool:${id}`,
        status: failed
          ? 'failed'
          : value.type === 'tool_execution_end'
            ? 'completed'
            : 'in_progress',
        title,
        toolKind: text(value.toolName) ?? null,
      });
      return;
    }
    if (value.type === 'agent_settled') {
      const turn = this.#turn;
      this.#turn = null;
      if (turn) turn.resolve('end_turn');
      else this.#handlers.onIdle();
    }
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

function piTokens(usage: Record<string, unknown> | null): number | null {
  if (!usage) return null;
  const total = positiveNumber(usage.totalTokens);
  if (total) return total;
  const values = [
    usage.input,
    usage.output,
    usage.cacheRead,
    usage.cacheWrite,
  ].filter((value): value is number => typeof value === 'number');
  return values.length ? values.reduce((sum, value) => sum + value, 0) : null;
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

function positiveNumber(value: unknown): number | null {
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
