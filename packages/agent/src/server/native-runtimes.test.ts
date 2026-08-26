import assert from 'node:assert/strict';
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import type { NativeAgentRuntimeHandlers } from './agent-runtime.js';
import { ClaudeRuntime } from './claude-runtime.js';
import { CodexRuntime } from './codex-runtime.js';
import { PiRuntime } from './pi-runtime.js';

void test('drives Claude stream-json across a turn', async () => {
  const harness = runtimeHarness('claude');
  const runtime = await ClaudeRuntime.start({
    binary: harness.executable,
    cwd: harness.directory,
    handlers: harness.handlers,
    models: [{ id: 'claude-test', isDefault: true, name: 'Claude Test' }],
    sessionId: null,
  });
  try {
    assert.equal(await runtime.prompt('hello'), 'end_turn');
    assert.deepEqual(harness.text, ['message:Claude OK']);
  } finally {
    runtime.close();
    harness.remove();
  }
});

void test('drives Codex app-server across a thread and turn', async () => {
  const harness = runtimeHarness('codex');
  const runtime = await CodexRuntime.start({
    binary: harness.executable,
    cwd: harness.directory,
    handlers: harness.handlers,
    models: [{ id: 'codex-test', isDefault: true, name: 'Codex Test' }],
    sessionId: null,
  });
  try {
    assert.equal(harness.sessionId, 'codex-thread');
    assert.equal(await runtime.prompt('hello'), 'end_turn');
    assert.deepEqual(harness.text, ['thought:checking', 'message:Codex OK']);
    assert.deepEqual(harness.titles, ['Codex title']);
  } finally {
    runtime.close();
    harness.remove();
  }
});

void test('drives Pi RPC and persists its session path', async () => {
  const harness = runtimeHarness('pi');
  const runtime = await PiRuntime.start({
    binary: harness.executable,
    cwd: harness.directory,
    handlers: harness.handlers,
    models: [{ id: 'provider/model', isDefault: true, name: 'Pi Test' }],
    selectedModel: 'provider/model',
    sessionPath: null,
  });
  try {
    assert.equal(harness.sessionId, 'pi-session');
    assert.equal(harness.sessionPath, '/sessions/pi.jsonl');
    assert.equal(await runtime.prompt('hello'), 'end_turn');
    assert.deepEqual(harness.text, ['thought:checking', 'message:Pi OK']);
  } finally {
    runtime.close();
    harness.remove();
  }
});

function runtimeHarness(provider: 'claude' | 'codex' | 'pi') {
  const directory = mkdtempSync(join(tmpdir(), `locus-${provider}-runtime-`));
  const executable = join(directory, provider);
  writeFileSync(executable, fakeRuntimeSource(provider));
  chmodSync(executable, 0o755);
  const text: string[] = [];
  const titles: string[] = [];
  let sessionId: string | null = null;
  let sessionPath: string | null = null;
  const handlers: NativeAgentRuntimeHandlers = {
    onCommands: () => undefined,
    onConnected: (id, _config, path) => {
      sessionId = id;
      sessionPath = path ?? null;
    },
    onError: (message) => assert.fail(message),
    onExit: () => {},
    onIdle: () => {},
    onPermission: () => assert.fail('unexpected permission'),
    onText: (kind, value) => text.push(`${kind}:${value}`),
    onTitle: (title) => titles.push(title),
    onTool: () => {},
    onUsage: () => {},
  };
  return {
    directory,
    executable,
    handlers,
    remove: () => rmSync(directory, { force: true, recursive: true }),
    get sessionId() {
      return sessionId;
    },
    get sessionPath() {
      return sessionPath;
    },
    text,
    titles,
  };
}

function fakeRuntimeSource(provider: 'claude' | 'codex' | 'pi'): string {
  return `#!/usr/bin/env node
import { createInterface } from 'node:readline';
const provider = ${JSON.stringify(provider)};
const send = (value) => process.stdout.write(JSON.stringify(value) + '\\n');
const lines = createInterface({ input: process.stdin });
lines.on('line', (line) => {
  const value = JSON.parse(line);
  if (provider === 'claude' && value.type === 'user') {
    send({ type: 'stream_event', event: { type: 'content_block_delta', delta: { type: 'text_delta', text: 'Claude OK' } } });
    send({ type: 'result', is_error: false });
    return;
  }
  if (provider === 'codex') {
    if (value.id === 1) {
      send({ id: 1, result: { thread: { id: 'codex-thread' } } });
    } else if (value.method === 'turn/start') {
      send({ method: 'turn/started', params: { turn: { id: 'turn-1' } } });
      send({ method: 'item/reasoning/summaryTextDelta', params: { delta: 'checking' } });
      send({ method: 'item/agentMessage/delta', params: { delta: 'Codex OK' } });
      send({ method: 'thread/name/updated', params: { threadId: 'codex-thread', threadName: 'Codex title' } });
      send({ method: 'turn/completed', params: { turn: { id: 'turn-1', status: 'completed' } } });
    }
    return;
  }
  if (provider === 'pi') {
    send({ type: 'response', id: value.id, success: true, data: value.type === 'get_state' ? { sessionId: 'pi-session', sessionFile: '/sessions/pi.jsonl', model: { contextWindow: 200000 } } : {} });
    if (value.type === 'prompt') {
      queueMicrotask(() => {
        send({ type: 'message_update', assistantMessageEvent: { type: 'thinking_delta', delta: 'checking' } });
        send({ type: 'message_update', assistantMessageEvent: { type: 'text_delta', delta: 'Pi OK' } });
        send({ type: 'agent_settled' });
      });
    }
  }
});
`;
}
