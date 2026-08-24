import { createMcpHandler } from '@modelcontextprotocol/server';
import { getLibrary } from '~/server/features/library/instance.server';
import { readGlobalSettings } from '~/server/features/settings/global-settings.server';
import { createAsterweftMcpServer } from './server';

interface McpRequestHandlerDependencies {
  getLibrary?: typeof getLibrary;
  readGlobalSettings?: typeof readGlobalSettings;
}

export function createAsterweftMcpRequestHandler(
  dependencies: McpRequestHandlerDependencies = {},
) {
  const resolveLibrary = dependencies.getLibrary ?? getLibrary;
  const resolveSettings = dependencies.readGlobalSettings ?? readGlobalSettings;
  const libraryHostPath = process.env.ASTERWEFT_LIBRARY_HOST_PATH?.trim();
  const handler = createMcpHandler(
    async () =>
      createAsterweftMcpServer(await resolveLibrary(), {
        rootPath: libraryHostPath || undefined,
      }),
    {
      legacy: 'stateless',
      responseMode: 'auto',
    },
  );

  return async (request: Request): Promise<Response> => {
    if (!resolveSettings().mcpEnabled) {
      return new Response('Not Found', {
        status: 404,
        statusText: 'Not Found',
      });
    }
    return handler.fetch(request);
  };
}

export const handleMcpRequest = createAsterweftMcpRequestHandler();
