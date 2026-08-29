import type { AgentWorkspaceEvent } from '@locus/agent/contracts/conversation';
import { getAgentService } from './service.server';

const encoder = new TextEncoder();
const HEARTBEAT_INTERVAL_MS = 5_000;

export function createAgentEventsResponse(request: Request): Response {
  const service = getAgentService();
  let cleanup = () => {};
  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      let closed = false;
      const close = () => {
        if (closed) return;
        closed = true;
        clearInterval(heartbeat);
        unsubscribe();
        request.signal.removeEventListener('abort', close);
        try {
          controller.close();
        } catch {
          return;
        }
      };
      const send = (event: AgentWorkspaceEvent) => {
        if (closed) return;
        try {
          controller.enqueue(
            encoder.encode(`data: ${JSON.stringify(event)}\n\n`),
          );
        } catch {
          close();
        }
      };
      const heartbeat = setInterval(() => {
        if (closed) return;
        try {
          controller.enqueue(encoder.encode(': keep-alive\n\n'));
        } catch {
          close();
        }
      }, HEARTBEAT_INTERVAL_MS);
      const unsubscribe = service.subscribe(send);
      request.signal.addEventListener('abort', close, { once: true });
      send({
        conversations: service.listConversations().conversations,
        type: 'snapshot',
      });
      cleanup = close;
    },
    cancel() {
      cleanup();
    },
  });
  return new Response(stream, {
    headers: {
      'Cache-Control': 'no-cache, no-transform',
      Connection: 'keep-alive',
      'Content-Type': 'text/event-stream; charset=utf-8',
      'X-Accel-Buffering': 'no',
    },
  });
}
