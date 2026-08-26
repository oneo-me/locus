import {
  parseClientMessage,
  type ServerMessage,
  type SessionHandle,
  type SessionState,
} from '@shellweb/core/protocol';
import {
  SessionError,
  TerminalSessionManager,
  type CreateTerminalSessionOptions,
  type TerminalAttachment,
} from '@shellweb/core/server';
import type { WSEvents } from 'hono/ws';
import type { WebSocketLike } from '@hono/node-server';
import { WebSocketServer } from 'ws';
import { getLibrary } from '~/features/library/instance.server';
import { readGlobalSettings } from '~/features/settings/global-settings.server';
import { logError, logInfo } from '~/logging';

const terminalSocketPath = '/api/terminal/socket';
export const terminalWebSocketServer = new WebSocketServer({
  maxPayload: 64 * 1024,
  noServer: true,
});

interface TerminalRuntime {
  manager: TerminalSessionManager;
  rootPath: string;
}

let runtime: TerminalRuntime | undefined;

async function getRuntime(): Promise<TerminalRuntime> {
  const library = await getLibrary();
  const rootPath = library.status().rootPath;
  const settings = readGlobalSettings();
  if (runtime?.rootPath === rootPath) return runtime;

  runtime?.manager.dispose();
  runtime = {
    manager: new TerminalSessionManager({
      cwd: rootPath,
      command: settings.terminalShell ?? undefined,
      detachedTtlMs: 30 * 60_000,
      maxSessions: 1,
      scrollback: 5_000,
    }),
    rootPath,
  };
  logInfo('terminal.runtime.started', { rootPath });
  return runtime;
}

export async function createTerminalSession(
  options: Pick<CreateTerminalSessionOptions, 'cols' | 'rows'>,
): Promise<SessionHandle> {
  const startupCommand = readGlobalSettings().terminalStartupCommand;
  return (await getRuntime()).manager.createSession({
    ...options,
    startupCommand: startupCommand || undefined,
  });
}

export async function getTerminalSession(
  sessionId: string,
  credential: string,
): Promise<SessionState> {
  return (await getRuntime()).manager.getSession(sessionId, credential);
}

export async function issueTerminalAttachment(
  sessionId: string,
  credential: string,
) {
  const ticket = (await getRuntime()).manager.issueAttachTicket(
    sessionId,
    credential,
  );
  return { ticket, websocketUrl: terminalSocketPath };
}

export async function deleteTerminalSession(
  sessionId: string,
  credential: string,
): Promise<void> {
  (await getRuntime()).manager.deleteSession(sessionId, credential);
}

export function resetTerminalSessions(): void {
  if (!runtime) return;
  runtime.manager.dispose();
  runtime = undefined;
  logInfo('terminal.runtime.stopped');
}

export function closeTerminalService(): void {
  resetTerminalSessions();
  for (const client of terminalWebSocketServer.clients) client.terminate();
  terminalWebSocketServer.close();
}

export function createTerminalWebSocketEvents(): WSEvents<WebSocketLike> {
  let attachment: TerminalAttachment | undefined;
  let messageQueue = Promise.resolve();
  let attachTimer: ReturnType<typeof setTimeout> | undefined;

  return {
    onOpen(_event, socket) {
      attachTimer = setTimeout(
        () => socket.close(4008, 'Attachment ticket timed out'),
        10_000,
      );
    },
    onMessage(event, socket) {
      messageQueue = messageQueue
        .then(async () => {
          const message =
            typeof event.data === 'string'
              ? parseClientMessage(event.data)
              : undefined;
          if (!message) {
            throw new SessionError(
              'invalid_message',
              'Invalid terminal protocol message.',
            );
          }
          if (message.type === 'attach') {
            if (attachment) {
              throw new SessionError(
                'already_attached',
                'This socket is already attached.',
              );
            }
            attachment = await (
              await getRuntime()
            ).manager.attachWithTicket(message.ticket, {
              send(value) {
                socket.send(JSON.stringify(value));
              },
              close(code, reason) {
                socket.close(code, reason);
              },
            });
            if (attachTimer) clearTimeout(attachTimer);
          } else if (!attachment) {
            throw new SessionError(
              'not_attached',
              'Attach before sending terminal input.',
            );
          } else if (message.type === 'ready') {
            await attachment.ready();
          } else if (message.type === 'input') {
            await attachment.input(message.data);
          } else if (message.type === 'resize') {
            await attachment.resize(message.cols, message.rows);
          }
        })
        .catch((error: unknown) => {
          const value =
            error instanceof SessionError
              ? error
              : new SessionError('internal_error', 'Terminal request failed.');
          if (!(error instanceof SessionError)) {
            logError('terminal.socket.failed', error);
          }
          if (socket.readyState === 1) {
            socket.send(
              JSON.stringify({
                type: 'error',
                code: value.code,
                message: value.message,
              } satisfies ServerMessage),
            );
          }
          socket.close(4003, value.message);
        });
    },
    onClose() {
      if (attachTimer) clearTimeout(attachTimer);
      void attachment?.close();
    },
    onError() {
      if (attachTimer) clearTimeout(attachTimer);
      void attachment?.close();
    },
  };
}

export function terminalCredential(value: string | undefined): string {
  if (!value?.startsWith('Bearer ')) {
    throw new SessionError('unauthorized', 'A session credential is required.');
  }
  return value.slice(7);
}

export function terminalErrorResponse(error: unknown): Response {
  if (!(error instanceof SessionError) && !(error instanceof SyntaxError)) {
    logError('terminal.request.failed', error);
  }
  const value =
    error instanceof SessionError
      ? error
      : error instanceof SyntaxError
        ? new SessionError(
            'invalid_request',
            'The terminal request is invalid.',
          )
        : new SessionError('internal_error', 'The terminal service failed.');
  const status =
    value.code === 'unauthorized'
      ? 401
      : value.code === 'session_not_found'
        ? 404
        : value.code === 'session_limit'
          ? 429
          : value.code === 'internal_error'
            ? 500
            : 400;
  return Response.json(
    { code: value.code, message: value.message },
    { status },
  );
}
