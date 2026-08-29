import assert from 'node:assert/strict';
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import type { SessionNotification } from '@agentclientprotocol/sdk';
import { AcpRuntime } from './acp-runtime.js';

void test('connects to an ACP process and streams a prompt response', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'locus-acp-runtime-'));
  const executable = join(directory, 'acp');
  writeFileSync(
    executable,
    fakeAgentSource(import.meta.resolve('@agentclientprotocol/sdk')),
  );
  chmodSync(executable, 0o755);
  const updates: SessionNotification[] = [];
  let connected:
    | Parameters<
        Parameters<typeof AcpRuntime.start>[0]['handlers']['onConnected']
      >
    | undefined;
  const runtime = await AcpRuntime.start({
    args: [executable],
    binary: process.execPath,
    cwd: directory,
    handlers: {
      onConnected: (...value) => {
        connected = value;
      },
      onError: (message) => assert.fail(message),
      onExit: () => {},
      onPermission: () => assert.fail('unexpected permission request'),
      onUpdate: (notification) => updates.push(notification),
    },
    sessionId: null,
  });
  try {
    assert.equal(connected?.[0], 'test-session');
    assert.equal(connected?.[1]?.currentId, 'ask');
    assert.equal(connected?.[2][0]?.category, 'model');
    await runtime.setMode('build');
    const config = await runtime.setConfig('model', 'model-b');
    assert.equal(config[0]?.currentValue, 'model-b');
    const model = await runtime.setModel('model', 'model-a', config);
    assert.equal(model[0]?.currentValue, 'model-a');
    await runtime.prompt('hello');
    assert.equal(updates.length, 1);
    assert.equal(updates[0]?.update.sessionUpdate, 'agent_message_chunk');
  } finally {
    runtime.close();
    rmSync(directory, { force: true, recursive: true });
  }
});

function fakeAgentSource(sdkUrl: string): string {
  return `
import { Readable, Writable } from 'node:stream';
import { agent, methods, ndJsonStream, PROTOCOL_VERSION } from ${JSON.stringify(sdkUrl)};

const configOptions = [{
  category: 'model',
  currentValue: 'model-a',
  id: 'model',
  name: 'Model',
  options: [
    { name: 'Model A', value: 'model-a' },
    { name: 'Model B', value: 'model-b' },
  ],
  type: 'select',
}];
const app = agent({ name: 'test-agent' })
  .onRequest(methods.agent.initialize, () => ({
    agentCapabilities: { loadSession: true },
    authMethods: [],
    protocolVersion: PROTOCOL_VERSION,
  }))
  .onRequest(methods.agent.session.new, () => ({
    configOptions,
    modes: {
      availableModes: [
        { id: 'ask', name: 'Ask' },
        { id: 'build', name: 'Build' },
      ],
      currentModeId: 'ask',
    },
    sessionId: 'test-session',
  }))
  .onRequest(methods.agent.session.setMode, () => ({}))
  .onRequest(methods.agent.session.setConfigOption, ({ params }) => ({
    configOptions: configOptions.map((option) => ({
      ...option,
      currentValue: params.value,
    })),
  }))
  .onRequest('session/set_model', (params) => params, () => ({}))
  .onRequest(methods.agent.session.prompt, async ({ client, params }) => {
    await client.notify(methods.client.session.update, {
      sessionId: params.sessionId,
      update: {
        content: { text: 'OK', type: 'text' },
        sessionUpdate: 'agent_message_chunk',
      },
    });
    return { stopReason: 'end_turn' };
  })
  .onNotification(methods.agent.session.cancel, () => {});
const stream = ndJsonStream(
  Writable.toWeb(process.stdout),
  Readable.toWeb(process.stdin),
);
const connection = app.connect(stream);
await connection.closed;
`;
}
