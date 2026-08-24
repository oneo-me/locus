import { handleMcpRequest } from '~/server/features/mcp/handler.server';

async function handle({ request }: { request: Request }) {
  return handleMcpRequest(request);
}

export const loader = handle;
export const action = handle;
