import assert from 'node:assert/strict';
import {
  chmodSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import type { AgentSessionConfig } from '@locus/agent/contracts/conversation';
import { OpenCodeRuntime } from './runtime.js';

void test('shares a native OpenCode server and consumes its session events', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'locus-opencode-runtime-'));
  const executable = join(directory, 'opencode');
  const starts = join(directory, 'starts');
  writeFileSync(executable, fakeOpenCodeSource(starts));
  chmodSync(executable, 0o755);
  const titles: string[] = [];
  const text: string[] = [];
  const permissions: Array<{ requestId: string }> = [];
  let config: AgentSessionConfig[] = [];
  const handlers = {
    onCommands: () => undefined,
    onConnected: (_sessionId: string, options: AgentSessionConfig[]) => {
      config = options;
    },
    onError: (message: string) => assert.fail(message),
    onExit: () => {},
    onIdle: () => {},
    onPermission: (permission: { requestId: string }) =>
      permissions.push(permission),
    onText: (kind: 'message' | 'thought', value: string) =>
      text.push(`${kind}:${value}`),
    onTitle: (title: string) => titles.push(title),
    onTool: () => {},
    onUsage: () => {},
  };
  const options = {
    binary: executable,
    cwd: directory,
    handlers,
    models: [
      {
        defaultReasoningEffort: null,
        id: 'provider/model-a',
        isDefault: true,
        name: 'Model A',
        reasoningEfforts: [],
      },
      {
        defaultReasoningEffort: null,
        id: 'provider/model-b',
        isDefault: false,
        name: 'Model B',
        reasoningEfforts: [],
      },
    ],
    sessionId: null,
  };
  const first = await OpenCodeRuntime.start(options);
  const second = await OpenCodeRuntime.start(options);
  try {
    assert.equal(readFileSync(starts, 'utf8'), '1');
    assert.equal(config[0]?.type, 'select');
    assert.equal(config[0]?.currentValue, 'provider/model-a');
    const stopReason = await first.prompt('hello');
    assert.equal(stopReason, 'end_turn');
    assert.deepEqual(text, ['thought:thinking', 'message:OK']);
    assert.deepEqual(titles, ['Native title']);
    const changed = await first.setModel('model', 'provider/model-b', config);
    assert.equal(changed[0]?.currentValue, 'provider/model-b');
    const permissionTurn = first.prompt('permission');
    await until(() => permissions.length === 1);
    assert.equal(first.respondPermission('permission-2', 'always'), true);
    assert.equal(await permissionTurn, 'end_turn');
    assert.equal(await first.prompt('permission again'), 'end_turn');
    assert.equal(permissions.length, 1);
  } finally {
    first.close();
    second.close();
    rmSync(directory, { force: true, recursive: true });
  }
});

function fakeOpenCodeSource(startsPath: string): string {
  return `#!/usr/bin/env node
import { appendFileSync } from 'node:fs';
import { createServer } from 'node:http';

appendFileSync(${JSON.stringify(startsPath)}, '1');
const port = Number(process.argv[process.argv.indexOf('--port') + 1]);
const streams = new Set();
const permissions = new Map();
let session = 0;
let prompts = 0;
const send = (value) => {
  const chunk = \`data:\${JSON.stringify(value)}\\n\\n\`;
  for (const stream of streams) stream.write(chunk);
};
const server = createServer((request, response) => {
  const path = request.url;
  if (path === '/event') {
    response.writeHead(200, {
      'cache-control': 'no-cache',
      connection: 'keep-alive',
      'content-type': 'text/event-stream',
    });
    response.write(': connected\\n\\n');
    streams.add(response);
    request.on('close', () => streams.delete(response));
    return;
  }
  if (path === '/global/health') return json(response, 200, { healthy: true });
  if (path === '/api/model') return json(response, 200, { data: [{ providerID: 'provider', id: 'model-a', limit: { context: 200000 } }] });
  if (path === '/permission') return json(response, 200, []);
  if (request.method === 'POST' && path === '/session') {
    session += 1;
    return json(response, 200, { id: \`session-\${session}\` });
  }
  if (request.method === 'PATCH' && path.startsWith('/session/')) {
    return json(response, 200, {});
  }
  if (request.method === 'POST' && path.endsWith('/prompt_async')) {
    response.writeHead(204);
    response.end();
    queueMicrotask(() => {
      const sessionID = path.split('/')[2];
      prompts += 1;
      if (prompts > 1) {
        const id = \`permission-\${prompts}\`;
        permissions.set(id, sessionID);
        send({ type: 'permission.requested', properties: { sessionID, id, permission: 'bash', patterns: ['git status'], always: ['git *'] } });
        return;
      }
      send({ type: 'message.part.delta', properties: { sessionID, field: 'reasoning', partID: 'reason', delta: 'thinking' } });
      send({ type: 'message.part.delta', properties: { sessionID, field: 'text', partID: 'answer', delta: 'OK' } });
      send({ type: 'session.updated', properties: { sessionID, info: { title: 'Native title' } } });
      send({ type: 'session.idle', properties: { sessionID } });
    });
    return;
  }
  if (request.method === 'POST' && path.startsWith('/permission/') && path.endsWith('/reply')) {
    const id = path.split('/')[2];
    const sessionID = permissions.get(id);
    permissions.delete(id);
    response.writeHead(204);
    response.end();
    queueMicrotask(() => {
      send({ type: 'permission.replied', properties: { sessionID, requestID: id } });
      send({ type: 'session.idle', properties: { sessionID } });
    });
    return;
  }
  if (request.method === 'POST' && path.endsWith('/abort')) {
    response.writeHead(204);
    response.end();
    return;
  }
  json(response, 404, { error: 'missing route' });
});
server.listen(port, '127.0.0.1');
const close = () => {
  for (const stream of streams) stream.destroy();
  process.exit(0);
};
process.on('SIGTERM', close);
process.on('SIGINT', close);

function json(response, status, value) {
  response.writeHead(status, { 'content-type': 'application/json' });
  response.end(JSON.stringify(value));
}
`;
}

async function until(predicate: () => boolean): Promise<void> {
  const deadline = Date.now() + 1_000;
  while (!predicate()) {
    if (Date.now() >= deadline) throw new Error('condition timed out');
    await new Promise((resolve) => setTimeout(resolve, 5));
  }
}
