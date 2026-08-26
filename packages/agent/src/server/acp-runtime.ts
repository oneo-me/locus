import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { Readable, Writable } from 'node:stream';
import {
  client,
  methods,
  ndJsonStream,
  PROTOCOL_VERSION,
  type AgentCapabilities,
  type ClientContext,
  type RequestPermissionResponse,
  type SessionConfigOption,
  type SessionModeState,
  type SessionNotification,
} from '@agentclientprotocol/sdk';
import type { AcpAgentProviderId } from '../contracts/provider.js';
import type {
  AgentSessionConfig,
  AgentSessionModeState,
} from '../contracts/conversation.js';
import { normalizeSessionConfig } from './session-config.js';

interface Deferred<T> {
  promise: Promise<T>;
  reject: (reason?: unknown) => void;
  resolve: (value: T) => void;
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

interface AcpRuntimeHandlers {
  onConnected: (
    sessionId: string,
    modes: AgentSessionModeState | null,
    configOptions: AgentSessionConfig[],
  ) => void;
  onError: (message: string) => void;
  onExit: () => void;
  onPermission: (permission: AcpPermissionRequest) => void;
  onUpdate: (notification: SessionNotification) => void;
}

export interface AcpPermissionRequest {
  options: Array<{ id: string; kind: string; name: string }>;
  requestId: string;
  title: string;
}

interface AcpRuntimeOptions {
  binary: string;
  cwd: string;
  handlers: AcpRuntimeHandlers;
  providerId: AcpAgentProviderId;
  sessionId: string | null;
}

interface EstablishedSession {
  configOptions?: SessionConfigOption[] | null;
  modes?: SessionModeState | null;
  sessionId: string;
}

const ACP_LAUNCH: Record<
  AcpAgentProviderId,
  { args: string[]; env?: Record<string, string> }
> = {
  cursor: { args: ['acp'] },
  fx: { args: ['acp'] },
  grok: {
    args: ['agent', 'stdio'],
    env: { GROK_OAUTH2_REFERRER: 'locus' },
  },
  kimi: { args: ['acp'] },
  opencode: { args: ['acp'] },
};

export class AcpRuntime {
  readonly #child: ChildProcessWithoutNullStreams;
  readonly #closed = deferred<void>();
  readonly #handlers: AcpRuntimeHandlers;
  readonly #providerId: AcpAgentProviderId;
  readonly #permissionResponses = new Map<
    string,
    (response: RequestPermissionResponse) => void
  >();
  readonly #ready = deferred<void>();
  #context: ClientContext | null = null;
  #closing = false;
  #errorReported = false;
  #sessionId: string | null = null;
  #stderr = '';

  private constructor(options: AcpRuntimeOptions) {
    this.#handlers = options.handlers;
    this.#providerId = options.providerId;
    const launch = ACP_LAUNCH[options.providerId];
    this.#child = spawn(options.binary, launch.args, {
      cwd: options.cwd,
      env: { ...process.env, ...launch.env },
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    this.#child.stderr.setEncoding('utf8');
    this.#child.stderr.on('data', (chunk: string) => {
      this.#stderr = `${this.#stderr}${chunk}`.slice(-16_384);
    });
    this.#child.once('exit', (code, signal) => {
      const detail = this.#stderr.trim();
      const reason = detail || `进程已退出（${signal ?? code ?? 'unknown'}）。`;
      this.#ready.reject(new Error(reason));
      if (!this.#closing) this.#reportError(reason);
      this.#closed.resolve();
    });
    this.#child.once('error', (error) => {
      this.#ready.reject(error);
      if (!this.#closing) this.#reportError(error);
      this.#closed.resolve();
    });
    void this.#connect(options)
      .catch((error) => {
        this.#ready.reject(error);
        if (!this.#closing) this.#reportError(error);
      })
      .finally(() => {
        this.#cancelPermissions();
        this.#handlers.onExit();
      });
  }

  static async start(options: AcpRuntimeOptions): Promise<AcpRuntime> {
    const runtime = new AcpRuntime(options);
    await runtime.#ready.promise;
    return runtime;
  }

  async prompt(content: string): Promise<string> {
    if (!this.#context || !this.#sessionId)
      throw new Error('ACP 会话尚未连接。');
    const response = await this.#context.request(methods.agent.session.prompt, {
      prompt: [{ type: 'text', text: content }],
      sessionId: this.#sessionId,
    });
    return response.stopReason;
  }

  async cancel(): Promise<void> {
    if (!this.#context || !this.#sessionId) return;
    await this.#context.notify(methods.agent.session.cancel, {
      sessionId: this.#sessionId,
    });
    this.#cancelPermissions();
  }

  async setMode(modeId: string): Promise<void> {
    if (!this.#context || !this.#sessionId)
      throw new Error('ACP 会话尚未连接。');
    await this.#context.request(methods.agent.session.setMode, {
      modeId,
      sessionId: this.#sessionId,
    });
  }

  async setConfig(
    configId: string,
    value: boolean | string,
  ): Promise<AgentSessionConfig[]> {
    if (!this.#context || !this.#sessionId)
      throw new Error('ACP 会话尚未连接。');
    const response = await this.#context.request(
      methods.agent.session.setConfigOption,
      {
        configId,
        sessionId: this.#sessionId,
        ...(typeof value === 'boolean'
          ? { type: 'boolean' as const, value }
          : { value }),
      },
    );
    return normalizeSessionConfig(response.configOptions);
  }

  async setModel(
    configId: string,
    modelId: string,
    configOptions: AgentSessionConfig[],
  ): Promise<AgentSessionConfig[]> {
    if (this.#providerId === 'cursor' || this.#providerId === 'fx') {
      return this.setConfig(configId, modelId);
    }
    if (!this.#context || !this.#sessionId)
      throw new Error('ACP 会话尚未连接。');
    await this.#context.request('session/set_model', {
      modelId,
      sessionId: this.#sessionId,
    });
    return configOptions.map((option) =>
      option.id === configId && option.type === 'select'
        ? { ...option, currentValue: modelId }
        : option,
    );
  }

  respondPermission(requestId: string, optionId: string | null): boolean {
    const respond = this.#permissionResponses.get(requestId);
    if (!respond) return false;
    this.#permissionResponses.delete(requestId);
    respond({
      outcome: optionId
        ? { outcome: 'selected', optionId }
        : { outcome: 'cancelled' },
    });
    return true;
  }

  close(): void {
    this.#closing = true;
    this.#closed.resolve();
    this.#cancelPermissions();
    if (!this.#child.killed) this.#child.kill();
  }

  async #connect(options: AcpRuntimeOptions): Promise<void> {
    const stream = ndJsonStream(
      Writable.toWeb(this.#child.stdin) as WritableStream<Uint8Array>,
      Readable.toWeb(this.#child.stdout) as ReadableStream<Uint8Array>,
    );
    const app = client({ name: 'locus' })
      .onNotification(methods.client.session.update, ({ params }) => {
        this.#handlers.onUpdate(params);
      })
      .onRequest(
        methods.client.session.requestPermission,
        ({ params, requestId }) =>
          new Promise<RequestPermissionResponse>((resolve) => {
            const requestIdText = String(requestId);
            this.#permissionResponses.set(requestIdText, resolve);
            this.#handlers.onPermission({
              options: params.options.map((option) => ({
                id: option.optionId,
                kind: option.kind,
                name: option.name,
              })),
              requestId: requestIdText,
              title: params.toolCall.title ?? '请求执行工具',
            });
          }),
      );

    await app.connectWith(stream, async (context) => {
      this.#context = context;
      const initialized = await context.request(methods.agent.initialize, {
        clientCapabilities: {
          terminal: false,
          ...(options.providerId === 'cursor'
            ? { _meta: { parameterizedModelPicker: true } }
            : {}),
        },
        clientInfo: { name: 'locus', title: 'Locus', version: '0.1.0' },
        protocolVersion: PROTOCOL_VERSION,
      });
      const session = await this.#establishSession(
        context,
        initialized.agentCapabilities,
        options.cwd,
        options.sessionId,
      );
      this.#sessionId = session.sessionId;
      this.#handlers.onConnected(
        session.sessionId,
        normalizeModes(session.modes),
        normalizeSessionConfig(session.configOptions),
      );
      this.#ready.resolve();
      await this.#closed.promise;
    });
  }

  async #establishSession(
    context: ClientContext,
    capabilities: AgentCapabilities | undefined,
    cwd: string,
    existingSessionId: string | null,
  ): Promise<EstablishedSession> {
    if (existingSessionId && capabilities?.sessionCapabilities?.resume) {
      const response = await context
        .request(methods.agent.session.resume, {
          cwd,
          mcpServers: [],
          sessionId: existingSessionId,
        })
        .catch(() => null);
      if (response) {
        return { ...response, sessionId: existingSessionId };
      }
    }
    if (existingSessionId && capabilities?.loadSession) {
      const response = await context
        .request(methods.agent.session.load, {
          cwd,
          mcpServers: [],
          sessionId: existingSessionId,
        })
        .catch(() => null);
      if (response) {
        return { ...response, sessionId: existingSessionId };
      }
    }
    const created = await context.request(methods.agent.session.new, {
      cwd,
      mcpServers: [],
    });
    return created;
  }

  #cancelPermissions(): void {
    for (const respond of this.#permissionResponses.values()) {
      respond({ outcome: { outcome: 'cancelled' } });
    }
    this.#permissionResponses.clear();
  }

  #reportError(error: unknown): void {
    if (this.#errorReported) return;
    this.#errorReported = true;
    this.#handlers.onError(
      error instanceof Error ? error.message : String(error),
    );
  }
}

function normalizeModes(
  modes: SessionModeState | null | undefined,
): AgentSessionModeState | null {
  if (!modes) return null;
  return {
    available: modes.availableModes.map(({ id, name }) => ({ id, name })),
    currentId: modes.currentModeId,
  };
}
