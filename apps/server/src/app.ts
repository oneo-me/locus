import { Hono } from 'hono';
import type { Context } from 'hono';
import type {
  ReorderDocumentsInput,
  UpdateDocumentIconImageInput,
} from '@locus/document-library/contracts/library-api';
import type { UpdateGlobalSettingsInput } from '@locus/document-library/contracts/global-settings';
import { createRouteAssetResponse } from '~/features/asset/route.server';
import { createWorkspaceEventsResponse } from '~/features/workspace/workspace-events';
import {
  getLibrarySelection,
  getLibraryWorkspaceView,
  reorderDocuments,
  updateDocumentIconImage,
} from '~/features/library/service';
import { handleMcpRequest } from '~/features/mcp/handler.server';
import {
  getGlobalSettings,
  updateGlobalSettings,
} from '~/features/settings/service';
import { jsonError, jsonOk } from '~/http/json';

async function jsonRoute<T>(operation: () => T | Promise<T>) {
  try {
    return jsonOk(await operation());
  } catch (error) {
    return jsonError(error);
  }
}

async function body<T>(context: Context): Promise<T> {
  return (await context.req.json()) as T;
}

export function createApp() {
  const app = new Hono();

  app.get('/api/health', (context) => context.json({ status: 'ok' as const }));
  app.get('/api/library', (context) =>
    jsonRoute(() => getLibraryWorkspaceView({ query: context.req.query('q') })),
  );
  app.get('/api/library/document', (context) =>
    jsonRoute(() =>
      getLibrarySelection({ doc: context.req.query('doc') ?? '' }),
    ),
  );
  app.post('/api/library/document/icon', async (context) =>
    jsonRoute(async () =>
      updateDocumentIconImage(
        await body<UpdateDocumentIconImageInput>(context),
      ),
    ),
  );
  app.post('/api/library/reorder', async (context) =>
    jsonRoute(async () =>
      reorderDocuments(await body<ReorderDocumentsInput>(context)),
    ),
  );
  app.get('/api/settings', () => jsonRoute(getGlobalSettings));
  app.patch('/api/settings', async (context) =>
    jsonRoute(async () =>
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

  app.notFound((context) => context.json({ error: 'Not Found' }, 404));
  app.onError((error) => jsonError(error, 500));

  return app;
}

export type LocusApp = ReturnType<typeof createApp>;
