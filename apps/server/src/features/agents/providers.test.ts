import assert from 'node:assert/strict';
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import type { NativeAgentRuntimeHandlers } from '@locus/agent/server/agent-runtime';
import { ClaudeRuntime } from '@locus/agent-provider-claude';
import { CodexRuntime } from '@locus/agent-provider-codex';
import { PiRuntime } from '@locus/agent-provider-pi';

void test('drives Claude stream-json across a turn', async () => {
  const harness = runtimeHarness('claude');
  const runtime = await ClaudeRuntime.start({
    binary: harness.executable,
    cwd: harness.directory,
    handlers: harness.handlers,
    models: [reasoningModel('claude-test', 'Claude Test')],
    selectedReasoningEffort: 'high',
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
    models: [reasoningModel('codex-test', 'Codex Test')],
    selectedReasoningEffort: 'high',
    sessionId: null,
  });
  try {
    assert.equal(harness.sessionId, 'codex-thread');
    await runtime.setConfig('reasoning_effort', 'low');
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
    models: [reasoningModel('provider/model', 'Pi Test')],
    selectedModel: 'provider/model',
    selectedReasoningEffort: 'high',
    sessionPath: null,
  });
  try {
    assert.equal(harness.sessionId, 'pi-session');
    assert.equal(harness.sessionPath, '/sessions/pi.jsonl');
    await runtime.setConfig('reasoning_effort', 'low');
    assert.equal(await runtime.prompt('hello'), 'end_turn');
    assert.deepEqual(harness.text, ['thought:checking', 'message:Pi OK']);
  } finally {
    runtime.close();
    harness.remove();
  }
});

function reasoningModel(id: string, name: string) {
  return {
    defaultReasoningEffort: 'medium',
    id,
    isDefault: true,
    name,
    reasoningEfforts: [
      { id: 'low', name: '低' },
      { id: 'medium', name: '中' },
      { id: 'high', name: '高' },
    ],
  };
}

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
let piEffort = null;
lines.on('line', (line) => {
  const value = JSON.parse(line);
  if (provider === 'claude' && value.type === 'user') {
    if (process.argv.at(process.argv.indexOf('--effort') + 1) !== 'high') {
      send({ type: 'result', is_error: true, result: 'wrong Claude effort' });
      return;
    }
    send({ type: 'stream_event', event: { type: 'content_block_delta', delta: { type: 'text_delta', text: 'Claude OK' } } });
    send({ type: 'result', is_error: false });
    return;
  }
  if (provider === 'codex') {
    if (value.id === 1) {
      send({ id: 1, result: { thread: { id: 'codex-thread' } } });
    } else if (value.method === 'turn/start') {
      if (value.params.effort !== 'low') {
        send({ method: 'turn/completed', params: { turn: { id: 'turn-1', status: 'failed', error: { message: 'wrong Codex effort' } } } });
        return;
      }
      send({ method: 'turn/started', params: { turn: { id: 'turn-1' } } });
      send({ method: 'item/reasoning/summaryTextDelta', params: { delta: 'checking' } });
      send({ method: 'item/agentMessage/delta', params: { delta: 'Codex OK' } });
      send({ method: 'thread/name/updated', params: { threadId: 'codex-thread', threadName: 'Codex title' } });
      send({ method: 'turn/completed', params: { turn: { id: 'turn-1', status: 'completed' } } });
    }
    return;
  }
  if (provider === 'pi') {
    if (value.type === 'set_thinking_level') piEffort = value.level;
    send({ type: 'response', id: value.id, success: true, data: value.type === 'get_state' ? { sessionId: 'pi-session', sessionFile: '/sessions/pi.jsonl', model: { contextWindow: 200000 } } : {} });
    if (value.type === 'prompt') {
      queueMicrotask(() => {
        send({ type: 'message_update', assistantMessageEvent: { type: 'thinking_delta', delta: 'checking' } });
        send({ type: 'message_update', assistantMessageEvent: { type: 'text_delta', delta: piEffort === 'low' ? 'Pi OK' : 'wrong Pi effort' } });
        send({ type: 'agent_settled' });
      });
    }
  }
});
`;
}
