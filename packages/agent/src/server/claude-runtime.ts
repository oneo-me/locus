import { randomUUID } from 'node:crypto';
import type { AgentSessionConfig } from '../contracts/conversation.js';
import type { AgentProviderModel } from '../contracts/provider.js';
import type {
  AgentRuntime,
  NativeAgentRuntimeHandlers,
} from './agent-runtime.js';
import { JsonLineProcess } from './json-line-process.js';

interface ClaudeRuntimeOptions {
  binary: string;
  cwd: string;
  handlers: NativeAgentRuntimeHandlers;
  models: AgentProviderModel[];
  selectedModel?: string;
  sessionId: string | null;
}

interface Deferred<T> {
  promise: Promise<T>;
  reject: (reason?: unknown) => void;
  resolve: (value: T) => void;
}

export class ClaudeRuntime implements AgentRuntime {
  readonly #handlers: NativeAgentRuntimeHandlers;
  readonly #models: AgentProviderModel[];
  readonly #process: JsonLineProcess;
  readonly #tools = new Map<string, { title: string; toolKind: string }>();
  #closed = false;
  #model: string | undefined;
  #requestId = 0;
  #sawTextDelta = false;
  #sawThoughtDelta = false;
  #turn: Deferred<string> | null = null;

  private constructor(options: ClaudeRuntimeOptions, sessionId: string) {
    this.#handlers = options.handlers;
    this.#models = options.models;
    this.#model = options.selectedModel ?? defaultModel(options.models);
    const args = [
      '-p',
      '--input-format',
      'stream-json',
      '--output-format',
      'stream-json',
      '--verbose',
      '--include-partial-messages',
      '--thinking-display',
      'summarized',
      '--replay-user-messages',
      '--permission-prompt-tool',
      'stdio',
      '--permission-mode',
      'default',
      ...(this.#model ? ['--model', this.#model] : []),
      ...(options.sessionId
        ? ['--resume', sessionId]
        : ['--session-id', sessionId]),
    ];
    this.#process = new JsonLineProcess({
      args,
      binary: options.binary,
      cwd: options.cwd,
      onExit: (message) => {
        if (message) this.#handlers.onError(message);
        this.#turn?.reject(new Error(message ?? 'Claude 进程已退出。'));
        this.#turn = null;
        this.#handlers.onExit();
      },
      onMessage: (message) => this.#handle(message),
    });
  }

  static async start(options: ClaudeRuntimeOptions): Promise<ClaudeRuntime> {
    const sessionId = options.sessionId ?? randomUUID();
    const runtime = new ClaudeRuntime(options, sessionId);
    options.handlers.onConnected(sessionId, runtime.#configOptions());
    return runtime;
  }

  async prompt(content: string): Promise<string> {
    if (this.#turn) throw new Error('Claude 仍在处理上一条消息。');
    const turn = deferred<string>();
    this.#turn = turn;
    this.#process.send({
      message: { content: [{ text: content, type: 'text' }], role: 'user' },
      parent_tool_use_id: null,
      type: 'user',
    });
    return turn.promise;
  }

  async cancel(): Promise<void> {
    this.#requestId += 1;
    this.#process.send({
      request: { subtype: 'interrupt' },
      request_id: `locus-${this.#requestId}`,
      type: 'control_request',
    });
  }

  async setMode(): Promise<void> {
    throw new Error('Claude runtime 不提供会话模式切换。');
  }

  async setConfig(
    configId: string,
    value: boolean | string,
  ): Promise<AgentSessionConfig[]> {
    if (configId !== 'model' || typeof value !== 'string') {
      throw new Error('Claude 会话配置无效。');
    }
    return this.setModel(configId, value, this.#configOptions());
  }

  async setModel(
    configId: string,
    modelId: string,
    _configOptions: AgentSessionConfig[],
  ): Promise<AgentSessionConfig[]> {
    if (
      configId !== 'model' ||
      !this.#models.some((model) => model.id === modelId)
    ) {
      throw new Error('Claude 模型无效。');
    }
    this.#model = modelId;
    this.#requestId += 1;
    this.#process.send({
      request: { model: modelId, subtype: 'set_model' },
      request_id: `locus-${this.#requestId}`,
      type: 'control_request',
    });
    return this.#configOptions();
  }

  respondPermission(requestId: string, optionId: string | null): boolean {
    if (!optionId) return false;
    this.#process.send({
      response: {
        request_id: requestId,
        response:
          optionId === 'deny'
            ? { behavior: 'deny', message: 'The user denied this tool call.' }
            : { behavior: 'allow' },
        subtype: 'success',
      },
      type: 'control_response',
    });
    return true;
  }

  close(): void {
    if (this.#closed) return;
    this.#closed = true;
    this.#turn?.reject(new Error('Claude 会话已关闭。'));
    this.#turn = null;
    this.#process.close();
  }

  #configOptions(): AgentSessionConfig[] {
    if (!this.#model || !this.#models.length) return [];
    return [
      {
        category: 'model',
        currentValue: this.#model,
        id: 'model',
        name: '模型',
        options: this.#models.map(({ id, name }) => ({ name, value: id })),
        type: 'select',
      },
    ];
  }

  #handle(value: Record<string, unknown>): void {
    if (value.parent_tool_use_id) return;
    if (value.type === 'stream_event') {
      const event = record(value.event);
      if (event?.type === 'message_start') {
        this.#sawTextDelta = false;
        this.#sawThoughtDelta = false;
      }
      const delta = record(event?.delta);
      if (delta?.type === 'text_delta' && typeof delta.text === 'string') {
        this.#sawTextDelta = true;
        this.#handlers.onText('message', delta.text);
      } else if (
        delta?.type === 'thinking_delta' &&
        typeof delta.thinking === 'string'
      ) {
        this.#sawThoughtDelta = true;
        this.#handlers.onText('thought', delta.thinking);
      }
      return;
    }
    if (value.type === 'assistant') {
      const message = record(value.message);
      const usage = record(message?.usage);
      if (usage) this.#handlers.onUsage(claudeTokens(usage), null);
      const content = Array.isArray(message?.content) ? message.content : [];
      for (const item of content) this.#handleAssistantBlock(item);
      return;
    }
    if (value.type === 'user' && value.isReplay !== true) {
      const content = record(value.message)?.content;
      if (Array.isArray(content)) {
        for (const item of content) this.#handleToolResult(item);
      }
      return;
    }
    if (value.type === 'control_request') {
      const request = record(value.request);
      const requestId = text(value.request_id);
      if (request?.subtype === 'can_use_tool' && requestId) {
        const title =
          text(record(request.input)?.command) ??
          text(request.display_name) ??
          text(request.tool_name) ??
          '运行工具';
        this.#handlers.onPermission({
          options: [
            { id: 'allow', kind: 'allow_once', name: '允许' },
            { id: 'deny', kind: 'reject_once', name: '拒绝' },
          ],
          requestId,
          title,
        });
      }
      return;
    }
    if (value.type === 'system' && value.subtype === 'init') {
      const commands = Array.isArray(value.slash_commands)
        ? value.slash_commands.filter(
            (item): item is string => typeof item === 'string',
          )
        : [];
      this.#handlers.onCommands(
        commands.map((name) => ({ description: '', name })),
      );
      return;
    }
    if (value.type === 'result') {
      const failed = value.is_error === true;
      const message = text(value.result);
      const turn = this.#turn;
      this.#turn = null;
      if (!turn) {
        this.#handlers.onIdle();
      } else if (failed) {
        turn.reject(new Error(message ?? 'Claude 回合失败。'));
      } else {
        turn.resolve('end_turn');
      }
    }
  }

  #handleAssistantBlock(value: unknown): void {
    const block = record(value);
    if (!block) return;
    if (
      block.type === 'text' &&
      !this.#sawTextDelta &&
      typeof block.text === 'string'
    ) {
      this.#handlers.onText('message', block.text);
    } else if (
      block.type === 'thinking' &&
      !this.#sawThoughtDelta &&
      typeof block.thinking === 'string'
    ) {
      this.#handlers.onText('thought', block.thinking);
    } else if (block.type === 'tool_use') {
      const id = text(block.id) ?? randomUUID();
      const toolKind = text(block.name) ?? 'tool';
      const input = record(block.input);
      const title =
        text(input?.command) ?? text(input?.description) ?? toolKind;
      this.#tools.set(id, { title, toolKind });
      this.#handlers.onTool({
        content: input ? JSON.stringify(input, null, 2) : null,
        id: `tool:${id}`,
        status: 'in_progress',
        title,
        toolKind,
      });
    }
  }

  #handleToolResult(value: unknown): void {
    const block = record(value);
    if (block?.type !== 'tool_result') return;
    const id = text(block.tool_use_id);
    if (!id) return;
    const stored = this.#tools.get(id) ?? {
      title: '工具调用',
      toolKind: 'tool',
    };
    this.#tools.delete(id);
    this.#handlers.onTool({
      content: jsonText(block.content),
      id: `tool:${id}`,
      status: block.is_error === true ? 'failed' : 'completed',
      title: stored.title,
      toolKind: stored.toolKind,
    });
  }
}

function defaultModel(models: AgentProviderModel[]): string | undefined {
  return models.find((model) => model.isDefault)?.id ?? models[0]?.id;
}

function claudeTokens(usage: Record<string, unknown>): number | null {
  const values = [
    usage.input_tokens,
    usage.output_tokens,
    usage.cache_creation_input_tokens,
    usage.cache_read_input_tokens,
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

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, reject, resolve };
}
