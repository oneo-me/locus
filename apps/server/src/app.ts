import { Hono } from 'hono';
import type { Context } from 'hono';
import { logger } from 'hono/logger';
import { upgradeWebSocket } from '@hono/node-server';
import { serveStatic } from '@hono/node-server/serve-static';
import type {
  OpenDocumentInput,
  ReorderDocumentsInput,
  SetDocumentArchivedInput,
  SetDocumentGraphParticipationInput,
  UpdateDocumentIconImageInput,
} from '@locus/document-library/contracts/library-api';
import type { UpdateGlobalSettingsInput } from '@locus/document-library/contracts/global-settings';
import type { CreateTerminalSessionOptions } from '@shellweb/core/server';
import { createRouteAssetResponse } from '~/features/asset/route.server';
import { createWorkspaceEventsResponse } from '~/features/workspace/workspace-events';
import {
  getLibrarySelection,
  getLibraryArchiveView,
  getLibraryGraph,
  getLibraryWorkspaceView,
  openDocument,
  reorderDocuments,
  setDocumentArchived,
  setDocumentGraphParticipation,
  updateDocumentIconImage,
} from '~/features/library/service';
import { handleMcpRequest } from '~/features/mcp/handler.server';
import {
  getGlobalSettings,
  updateGlobalSettings,
} from '~/features/settings/service';
import { jsonError, jsonOk } from '~/http/json';
import { logError } from '~/logging';
import {
  createTerminalSession,
  createTerminalWebSocketEvents,
  deleteTerminalSession,
  getTerminalSession,
  issueTerminalAttachment,
  terminalCredential,
  terminalErrorResponse,
} from '~/features/terminal/service.server';

async function jsonRoute<T>(event: string, operation: () => T | Promise<T>) {
  try {
    return jsonOk(await operation());
  } catch (error) {
    logError(`${event}.failed`, error);
    return jsonError(error);
  }
}

async function body<T>(context: Context): Promise<T> {
  return (await context.req.json()) as T;
}

function isServerPath(path: string): boolean {
  return (
    path === '/api' ||
    path.startsWith('/api/') ||
    path === '/mcp' ||
    path.startsWith('/mcp/')
  );
}

export function createApp(staticRoot?: string) {
  const app = new Hono();

  app.use(logger());

  app.get('/api/health', (context) => context.json({ status: 'ok' as const }));
  app.get(
    '/api/terminal/socket',
    upgradeWebSocket(() => createTerminalWebSocketEvents()),
  );
  app.get('/api/library', () =>
    jsonRoute('library.workspace.read', () => getLibraryWorkspaceView()),
  );
  app.get('/api/library/graph', () =>
    jsonRoute('library.graph.read', () => getLibraryGraph()),
  );
  app.get('/api/library/archives', () =>
    jsonRoute('library.archives.read', () => getLibraryArchiveView()),
  );
  app.get('/api/library/document', (context) =>
    jsonRoute('library.document.read', () =>
      getLibrarySelection({ doc: context.req.query('doc') ?? '' }),
    ),
  );
  app.post('/api/library/document/icon', async (context) =>
    jsonRoute('library.document_icon.update', async () =>
      updateDocumentIconImage(
        await body<UpdateDocumentIconImageInput>(context),
      ),
    ),
  );
  app.post('/api/library/document/open', async (context) =>
    jsonRoute('library.document.open', async () =>
      openDocument(await body<OpenDocumentInput>(context)),
    ),
  );
  app.post('/api/terminal/sessions', async (context) => {
    try {
      return Response.json(
        await createTerminalSession(
          await body<Pick<CreateTerminalSessionOptions, 'cols' | 'rows'>>(
            context,
          ),
        ),
        { status: 201 },
      );
    } catch (error) {
      return terminalErrorResponse(error);
    }
  });
  app.get('/api/terminal/sessions/:id', async (context) => {
    try {
      return Response.json(
        await getTerminalSession(
          context.req.param('id'),
          terminalCredential(context.req.header('authorization')),
        ),
      );
    } catch (error) {
      return terminalErrorResponse(error);
    }
  });
  app.post('/api/terminal/sessions/:id/attachments', async (context) => {
    try {
      return Response.json(
        await issueTerminalAttachment(
          context.req.param('id'),
          terminalCredential(context.req.header('authorization')),
        ),
        { status: 201 },
      );
    } catch (error) {
      return terminalErrorResponse(error);
    }
  });
  app.delete('/api/terminal/sessions/:id', async (context) => {
    try {
      await deleteTerminalSession(
        context.req.param('id'),
        terminalCredential(context.req.header('authorization')),
      );
      return new Response(null, { status: 204 });
    } catch (error) {
      return terminalErrorResponse(error);
    }
  });
  app.post('/api/library/reorder', async (context) =>
    jsonRoute('library.documents.reorder', async () =>
      reorderDocuments(await body<ReorderDocumentsInput>(context)),
    ),
  );
  app.post('/api/library/graph-participation', async (context) =>
    jsonRoute('library.graph_participation.update', async () =>
      setDocumentGraphParticipation(
        await body<SetDocumentGraphParticipationInput>(context),
      ),
    ),
  );
  app.post('/api/library/archive', async (context) =>
    jsonRoute('library.archive.update', async () =>
      setDocumentArchived(await body<SetDocumentArchivedInput>(context)),
    ),
  );
  app.get('/api/settings', () => jsonRoute('settings.read', getGlobalSettings));
  app.patch('/api/settings', async (context) =>
    jsonRoute('settings.update', async () =>
      updateGlobalSettings(await body<UpdateGlobalSettingsInput>(context)),
    ),
  );
  app.get('/api/workspace-events', (context) =>
    createWorkspaceEventsResponse(context.req.raw),
  );
  app.get('/api/assets', async (context) => {
    const path = context.req.query('path');
    if (!path) return context.text('缺少或无法识别附件路径。', 400);
    return (
      (await createRouteAssetResponse(context.req.raw, path)) ??
      context.text('附件不存在。', 404)
    );
  });
  app.all('/mcp', (context) => handleMcpRequest(context.req.raw));

  if (staticRoot) {
    const serveClient = serveStatic({ root: staticRoot });
    const serveClientFallback = serveStatic({
      root: staticRoot,
      path: 'index.html',
    });

    app.use('/assets/*', async (context, next) => {
      const response = await serveClient(context, next);
      if (response?.ok) {
        response.headers.set(
          'Cache-Control',
          'public, max-age=31536000, immutable',
        );
      }
      return response;
    });
    app.use('*', async (context, next) => {
      if (
        isServerPath(context.req.path) ||
        context.req.path.startsWith('/assets/') ||
        (context.req.method !== 'GET' && context.req.method !== 'HEAD')
      ) {
        return next();
      }
      context.header('Cache-Control', 'no-cache');
      return serveClient(context, next);
    });
    app.get('*', async (context, next) => {
      if (
        isServerPath(context.req.path) ||
        context.req.path.startsWith('/assets/')
      ) {
        return next();
      }
      context.header('Cache-Control', 'no-cache');
      return serveClientFallback(context, next);
    });
  }

  app.notFound((context) => context.json({ error: 'Not Found' }, 404));
  app.onError((error) => {
    logError('http.request.failed', error);
    return jsonError(error, 500);
  });

  return app;
}

export type LocusApp = ReturnType<typeof createApp>;
