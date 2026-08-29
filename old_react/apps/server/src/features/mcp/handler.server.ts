import { createMcpHandler } from '@modelcontextprotocol/server';
import { getLibrary } from '~/features/library/instance.server';
import { readGlobalSettings } from '~/features/settings/global-settings.server';
import { createLocusMcpServer } from './server';

const handler = createMcpHandler(
  async () => createLocusMcpServer(await getLibrary()),
  {
    legacy: 'stateless',
    responseMode: 'auto',
  },
);

export async function handleMcpRequest(request: Request): Promise<Response> {
  if (!readGlobalSettings().mcpEnabled) {
    return new Response('Not Found', {
      status: 404,
      statusText: 'Not Found',
    });
  }
  return handler.fetch(request);
}
