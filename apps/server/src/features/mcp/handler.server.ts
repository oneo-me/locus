import { createMcpHandler } from '@modelcontextprotocol/server';
import { getLibrary } from '~/features/library/instance.server';
import { readGlobalSettings } from '~/features/settings/global-settings.server';
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
  const handler = createMcpHandler(
    async () => createAsterweftMcpServer(await resolveLibrary()),
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
