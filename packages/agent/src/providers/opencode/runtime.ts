import type { AgentSessionConfig } from '../../contracts/conversation.js';
import type { AgentProviderModel } from '../../contracts/provider.js';
import type {
  AgentRuntime,
  NativeAgentRuntimeHandlers,
} from '../../server/agent-runtime.js';
import { acquireOpenCodeServer, type OpenCodeServerLease } from './server.js';

interface Deferred<T> {
  promise: Promise<T>;
  reject: (reason?: unknown) => void;
  resolve: (value: T) => void;
}

interface OpenCodeRuntimeOptions {
  binary: string;
  cwd: string;
  handlers: NativeAgentRuntimeHandlers;
  models: AgentProviderModel[];
  selectedModel?: string;
  sessionId: string | null;
}

interface NativePermission {
  always: string[];
  patterns: string[];
  permission: string;
}

export class OpenCodeRuntime implements AgentRuntime {
  readonly #abort = new AbortController();
  readonly #approved = new Set<string>();
  readonly #handlers: NativeAgentRuntimeHandlers;
  readonly #contextWindows = new Map<string, number>();
  readonly #lease: OpenCodeServerLease;
  readonly #models: AgentProviderModel[];
  readonly #pendingPermissions = new Map<string, NativePermission>();
  readonly #reasoningParts = new Set<string>();
  readonly #sessionId: string;
  #closed = false;
  #model: string | undefined;
  #lastModel: string | undefined;
  #turn: Deferred<string> | null = null;

  private constructor(
    options: OpenCodeRuntimeOptions,
    lease: OpenCodeServerLease,
    sessionId: string,
  ) {
    this.#handlers = options.handlers;
    this.#lease = lease;
    this.#models = options.models;
    this.#model = options.selectedModel ?? defaultModel(options.models);
    this.#sessionId = sessionId;
  }

  static async start(
    options: OpenCodeRuntimeOptions,
  ): Promise<OpenCodeRuntime> {
    const lease = await acquireOpenCodeServer(options.binary, options.cwd);
    try {
      const sessionId =
        options.sessionId ?? (await createSession(lease, 'build'));
      await lease.server.request(
        'PATCH',
        `/session/${encodeURIComponent(sessionId)}`,
        {
          permission: [
            { action: 'ask', pattern: '*', permission: 'bash' },
            { action: 'ask', pattern: '*', permission: 'edit' },
          ],
        },
      );
      const runtime = new OpenCodeRuntime(options, lease, sessionId);
      await runtime.#openEvents();
      options.handlers.onConnected(sessionId, runtime.#configOptions());
      await runtime.#rehydratePermissions();
      void runtime.#loadPreviousUsage();
      void runtime.#loadUsageMetadata();
      return runtime;
    } catch (error) {
      lease.release();
      throw error;
    }
  }

  async prompt(content: string): Promise<string> {
    if (this.#closed) throw new Error('OpenCode 会话已关闭。');
    if (this.#turn) throw new Error('OpenCode 仍在处理上一条消息。');
    const turn = deferred<string>();
    this.#turn = turn;
    const body: Record<string, unknown> = {
      agent: 'build',
      parts: [{ text: content, type: 'text' }],
    };
    const model = splitModel(this.#model);
    if (model) body.model = model;
    try {
      await this.#lease.server.request(
        'POST',
        `/session/${encodeURIComponent(this.#sessionId)}/prompt_async`,
        body,
      );
    } catch (error) {
      this.#turn = null;
      turn.reject(error);
    }
    return turn.promise;
  }

  async cancel(): Promise<void> {
    if (this.#closed) return;
    await this.#lease.server.request(
      'POST',
      `/session/${encodeURIComponent(this.#sessionId)}/abort`,
    );
    this.#settleTurn('cancelled');
    this.#pendingPermissions.clear();
  }

  async setMode(): Promise<void> {
    throw new Error('OpenCode 原生 runtime 不提供会话模式切换。');
  }

  async setConfig(
    configId: string,
    value: boolean | string,
  ): Promise<AgentSessionConfig[]> {
    if (configId !== 'model' || typeof value !== 'string') {
      throw new Error('OpenCode 会话配置无效。');
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
      throw new Error('OpenCode 模型无效。');
    }
    this.#model = modelId;
    return this.#configOptions();
  }

  respondPermission(requestId: string, optionId: string | null): boolean {
    const request = this.#pendingPermissions.get(requestId);
    if (!request || !optionId) return false;
    this.#pendingPermissions.delete(requestId);
    const responses = [requestId];
    if (optionId === 'always') {
      for (const pattern of request.always) {
        this.#approved.add(permissionRule(request.permission, pattern));
      }
      for (const [pendingId, pending] of this.#pendingPermissions) {
        if (!this.#isApproved(pending)) continue;
        this.#pendingPermissions.delete(pendingId);
        responses.push(pendingId);
      }
    }
    const reply = optionId === 'always' ? 'once' : optionId;
    for (const id of responses) {
      void this.#replyPermission(id, reply).catch((error) =>
        this.#reportError(error),
      );
    }
    return true;
  }

  close(): void {
    if (this.#closed) return;
    this.#closed = true;
    this.#abort.abort();
    this.#turn?.reject(new Error('OpenCode 会话已关闭。'));
    this.#turn = null;
    this.#pendingPermissions.clear();
    this.#lease.release();
  }

  #configOptions(): AgentSessionConfig[] {
    if (!this.#model || this.#models.length === 0) return [];
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

  async #openEvents(): Promise<void> {
    const response = await fetch(`${this.#lease.server.baseUrl}/event`, {
      headers: { accept: 'text/event-stream' },
      signal: this.#abort.signal,
    });
    if (!response.ok || !response.body) {
      throw new Error(`OpenCode 事件流连接失败（HTTP ${response.status}）。`);
    }
    void this.#readEvents(response.body).catch((error) => {
      if (this.#closed || this.#abort.signal.aborted) return;
      this.#reportError(error);
      this.close();
      this.#handlers.onExit();
    });
  }

  async #readEvents(body: ReadableStream<Uint8Array>): Promise<void> {
    const reader = body.getReader();
    const decoder = new TextDecoder();
    let buffer = '';
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split(/\r?\n/);
      buffer = lines.pop() ?? '';
      for (const line of lines) {
        if (!line.startsWith('data:')) continue;
        try {
          this.#handleEvent(JSON.parse(line.slice(5).trim()) as unknown);
        } catch {
          continue;
        }
      }
    }
    if (!this.#closed) throw new Error('OpenCode 事件流已断开。');
  }

  #handleEvent(value: unknown): void {
    if (!isRecord(value)) return;
    const type = stringValue(value.type);
    const properties = isRecord(value.properties) ? value.properties : {};
    const eventSessionId = stringValue(properties.sessionID);
    if (eventSessionId && eventSessionId !== this.#sessionId) return;

    if (type === 'message.part.delta') {
      const delta = stringValue(properties.delta);
      const field = stringValue(properties.field);
      const partId = stringValue(properties.partID);
      if (!delta) return;
      if (field === 'reasoning' || field === 'thinking') {
        if (partId) this.#reasoningParts.add(partId);
        this.#handlers.onText('thought', delta);
      } else if (field === 'text') {
        this.#handlers.onText(
          partId && this.#reasoningParts.has(partId) ? 'thought' : 'message',
          delta,
        );
      }
      return;
    }
    if (type === 'message.part.updated') {
      const part = isRecord(properties.part) ? properties.part : null;
      if (!part) return;
      const partId = stringValue(part.id);
      const partType = stringValue(part.type);
      if (partId) {
        if (partType === 'reasoning' || partType === 'thinking') {
          this.#reasoningParts.add(partId);
        } else {
          this.#reasoningParts.delete(partId);
        }
      }
      if (partType === 'tool') this.#handleTool(part);
      return;
    }
    if (type === 'message.updated') {
      const info = isRecord(properties.info) ? properties.info : null;
      if (info && info.role === 'assistant') {
        const providerId = stringValue(info.providerID);
        const modelId = stringValue(info.modelID);
        this.#lastModel =
          providerId && modelId ? `${providerId}/${modelId}` : this.#lastModel;
        this.#handlers.onUsage(
          contextTokens(info),
          this.#lastModel
            ? (this.#contextWindows.get(this.#lastModel) ?? null)
            : null,
        );
      }
      return;
    }
    if (type === 'session.idle') {
      this.#reasoningParts.clear();
      this.#pendingPermissions.clear();
      if (this.#turn) this.#settleTurn('end_turn');
      else this.#handlers.onIdle();
      return;
    }
    if (type === 'session.error') {
      const error = isRecord(properties.error) ? properties.error : null;
      const message =
        stringValue(error?.message) ?? stringValue(properties.message);
      if (this.#turn) {
        this.#turn.reject(new Error(message ?? 'OpenCode 返回错误。'));
        this.#turn = null;
      } else {
        this.#reportError(message ?? 'OpenCode 返回错误。');
      }
      return;
    }
    if (type === 'session.updated') {
      const info = isRecord(properties.info) ? properties.info : null;
      const title = stringValue(info?.title)?.trim();
      if (title && !title.startsWith('New session - ')) {
        this.#handlers.onTitle(title);
      }
      return;
    }
    if (type === 'permission.replied') {
      const id =
        stringValue(properties.requestID) ?? stringValue(properties.id);
      if (id) this.#pendingPermissions.delete(id);
      return;
    }
    if (type?.startsWith('permission.')) this.#handlePermission(properties);
  }

  #handleTool(part: Record<string, unknown>): void {
    const state = isRecord(part.state) ? part.state : {};
    const status = stringValue(state.status);
    const failed = status === 'error' || state.error != null;
    const complete = status === 'completed' || failed;
    const input =
      state.input == null ? null : JSON.stringify(state.input, null, 2);
    const outputValue = state.error ?? state.output;
    const output =
      outputValue == null
        ? null
        : typeof outputValue === 'string'
          ? outputValue
          : JSON.stringify(outputValue, null, 2);
    this.#handlers.onTool({
      content: [input, output].filter(Boolean).join('\n') || null,
      id: `tool:${stringValue(part.callID) ?? stringValue(part.id) ?? 'unknown'}`,
      status: failed ? 'failed' : complete ? 'completed' : 'in_progress',
      title: stringValue(part.tool) ?? '工具调用',
      toolKind: stringValue(part.tool) ?? null,
    });
  }

  #handlePermission(properties: Record<string, unknown>): void {
    const nested = ['permission', 'request', 'info']
      .map((key) => properties[key])
      .find(
        (candidate) => isRecord(candidate) && typeof candidate.id === 'string',
      );
    const request = isRecord(nested) ? nested : properties;
    const requestId = stringValue(request.id);
    if (!requestId || this.#pendingPermissions.has(requestId)) return;
    const permission: NativePermission = {
      always: stringArray(request.always),
      patterns: stringArray(request.patterns),
      permission: stringValue(request.permission) ?? '',
    };
    if (this.#isApproved(permission)) {
      void this.#replyPermission(requestId, 'once').catch((error) =>
        this.#reportError(error),
      );
      return;
    }
    this.#pendingPermissions.set(requestId, permission);
    this.#handlers.onPermission({
      options: [
        { id: 'once', kind: 'allow_once', name: '允许一次' },
        { id: 'always', kind: 'allow_always', name: '始终允许' },
        { id: 'reject', kind: 'reject_once', name: '拒绝' },
      ],
      requestId,
      title:
        permission.patterns.join(', ') ||
        `允许 ${permission.permission || '工具调用'}`,
    });
  }

  async #rehydratePermissions(): Promise<void> {
    const response = await this.#lease.server.request('GET', '/permission');
    if (!Array.isArray(response)) return;
    for (const request of response) {
      if (isRecord(request) && request.sessionID === this.#sessionId) {
        this.#handlePermission(request);
      }
    }
  }

  async #loadUsageMetadata(): Promise<void> {
    const startedAt = Date.now();
    while (!this.#closed && Date.now() - startedAt < 30_000) {
      try {
        const response = await this.#lease.server.request('GET', '/api/model');
        const models =
          isRecord(response) && Array.isArray(response.data)
            ? response.data
            : null;
        if (models?.length) {
          for (const model of models) {
            if (!isRecord(model) || !isRecord(model.limit)) continue;
            const providerId = stringValue(model.providerID);
            const modelId = stringValue(model.id);
            const context = model.limit.context;
            if (
              providerId &&
              modelId &&
              typeof context === 'number' &&
              context > 0
            ) {
              this.#contextWindows.set(`${providerId}/${modelId}`, context);
            }
          }
          if (this.#lastModel) {
            const window = this.#contextWindows.get(this.#lastModel);
            if (window) this.#handlers.onUsage(null, window);
          }
          return;
        }
      } catch {
        if (this.#closed) return;
      }
      await abortableDelay(2_000, this.#abort.signal);
    }
  }

  async #loadPreviousUsage(): Promise<void> {
    try {
      const response = await this.#lease.server.request(
        'GET',
        `/session/${encodeURIComponent(this.#sessionId)}/message?limit=20`,
      );
      if (!Array.isArray(response)) return;
      const info = [...response]
        .reverse()
        .map((message) =>
          isRecord(message) && isRecord(message.info) ? message.info : null,
        )
        .find((candidate) => candidate?.role === 'assistant');
      if (!info) return;
      const providerId = stringValue(info.providerID);
      const modelId = stringValue(info.modelID);
      if (providerId && modelId) this.#lastModel = `${providerId}/${modelId}`;
      const tokens = contextTokens(info);
      if (tokens !== null) this.#handlers.onUsage(tokens, null);
    } catch {
      return;
    }
  }

  #isApproved(request: NativePermission): boolean {
    return (
      request.patterns.length > 0 &&
      request.patterns.every((pattern) =>
        [...this.#approved].some((rule) => {
          const separator = rule.indexOf('\0');
          return (
            wildcardMatches(request.permission, rule.slice(0, separator)) &&
            wildcardMatches(pattern, rule.slice(separator + 1))
          );
        }),
      )
    );
  }

  async #replyPermission(requestId: string, reply: string): Promise<void> {
    await this.#lease.server.request(
      'POST',
      `/permission/${encodeURIComponent(requestId)}/reply`,
      { reply },
    );
  }

  #settleTurn(reason: string): void {
    const turn = this.#turn;
    if (!turn) return;
    this.#turn = null;
    turn.resolve(reason);
  }

  #reportError(error: unknown): void {
    this.#handlers.onError(
      error instanceof Error ? error.message : String(error),
    );
  }
}

async function createSession(
  lease: OpenCodeServerLease,
  agent: string,
): Promise<string> {
  const response = await lease.server.request('POST', '/session', { agent });
  if (!isRecord(response) || typeof response.id !== 'string') {
    throw new Error('OpenCode 未返回 session ID。');
  }
  return response.id;
}

function defaultModel(models: AgentProviderModel[]): string | undefined {
  return models.find((model) => model.isDefault)?.id ?? models[0]?.id;
}

function splitModel(model: string | undefined): Record<string, string> | null {
  if (!model) return null;
  const separator = model.indexOf('/');
  if (separator <= 0 || separator === model.length - 1) return null;
  return {
    modelID: model.slice(separator + 1),
    providerID: model.slice(0, separator),
  };
}

function contextTokens(info: Record<string, unknown>): number | null {
  const tokens = isRecord(info.tokens) ? info.tokens : null;
  if (!tokens) return null;
  if (typeof tokens.total === 'number' && tokens.total > 0) return tokens.total;
  const values = ['input', 'output']
    .map((key) => tokens[key])
    .filter((value): value is number => typeof value === 'number');
  const cache = isRecord(tokens.cache) ? tokens.cache : null;
  if (cache) {
    for (const key of ['read', 'write']) {
      if (typeof cache[key] === 'number') values.push(cache[key]);
    }
  }
  return values.length ? values.reduce((sum, value) => sum + value, 0) : null;
}

function abortableDelay(
  milliseconds: number,
  signal: AbortSignal,
): Promise<void> {
  return new Promise((resolve) => {
    if (signal.aborted) {
      resolve();
      return;
    }
    const timeout = setTimeout(done, milliseconds);
    timeout.unref();
    signal.addEventListener('abort', done, { once: true });
    function done() {
      clearTimeout(timeout);
      signal.removeEventListener('abort', done);
      resolve();
    }
  });
}

function permissionRule(permission: string, pattern: string): string {
  return `${permission}\0${pattern}`;
}

function wildcardMatches(rawInput: string, rawPattern: string): boolean {
  const input = rawInput.replaceAll('\\', '/');
  const pattern = rawPattern.replaceAll('\\', '/');
  if (pattern.endsWith(' *') && input === pattern.slice(0, -2)) return true;
  let previous = Array.from(
    { length: input.length + 1 },
    (_, index) => index === 0,
  );
  for (const token of pattern) {
    const current = Array<boolean>(input.length + 1).fill(false);
    if (token === '*') current[0] = previous[0];
    for (let index = 1; index <= input.length; index += 1) {
      current[index] =
        token === '*'
          ? previous[index] || current[index - 1]
          : token === '?'
            ? previous[index - 1]
            : previous[index - 1] && token === input[index - 1];
    }
    previous = current;
  }
  return previous[input.length];
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function stringValue(value: unknown): string | undefined {
  return typeof value === 'string' ? value : undefined;
}

function stringArray(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === 'string')
    : [];
}
