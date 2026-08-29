import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import type { AgentConversation } from '../contracts/conversation.js';
import { AgentConversationStore } from './conversation-store.js';

void test('stores dated conversation metadata and replays JSONL deltas', () => {
  const root = mkdtempSync(join(tmpdir(), 'locus-conversation-store-'));
  const createdAt = new Date(2026, 7, 11, 9, 30).toISOString();
  const initial = conversation(createdAt);
  initial.providerSessionId = 'native-session';

  try {
    const store = new AgentConversationStore(root);
    store.save(initial);
    assert.equal(store.list().length, 1);
    assert.equal(store.list()[0]?.activityAt, createdAt);
    assert.equal(store.get(initial.id)?.providerSessionId, 'native-session');
    const directory = join(root, 'projects', '2026-08-11', initial.id);
    assert.equal(existsSync(join(directory, 'metadata.json')), true);
    assert.equal(existsSync(join(directory, 'transcript.jsonl')), true);

    const current = store.get(initial.id);
    assert.ok(current);
    current.transcript.push({
      content: 'O',
      createdAt,
      id: 'assistant',
      kind: 'message',
      role: 'assistant',
      streaming: true,
    });
    store.save(current);
    const assistant = current.transcript[1];
    assert.ok(assistant?.kind === 'message');
    assistant.content = 'OK';
    store.save(current);
    assistant.streaming = false;
    store.save(current);

    const lines = readFileSync(join(directory, 'transcript.jsonl'), 'utf8')
      .trim()
      .split('\n')
      .map((line) => JSON.parse(line) as { content?: string; op: string });
    assert.deepEqual(
      lines.slice(-3).map(({ content, op }) => ({ content, op })),
      [
        { content: undefined, op: 'put' },
        { content: 'K', op: 'append' },
        { content: '', op: 'append' },
      ],
    );

    const restored = new AgentConversationStore(root).get(initial.id);
    assert.ok(restored);
    const restoredAssistant = restored?.transcript[1];
    assert.ok(restoredAssistant?.kind === 'message');
    assert.equal(restoredAssistant.content, 'OK');
    assert.equal(restoredAssistant.streaming, false);

    restoredAssistant.content = 'OK!';
    const restoredStore = new AgentConversationStore(root);
    restoredStore.save(restored);
    const replayed = new AgentConversationStore(root).get(initial.id);
    const replayedAssistant = replayed?.transcript[1];
    assert.ok(replayedAssistant?.kind === 'message');
    assert.equal(replayedAssistant.content, 'OK!');
  } finally {
    rmSync(root, { force: true, recursive: true });
  }
});

function conversation(createdAt: string): AgentConversation {
  return {
    activityAt: createdAt,
    autoTitle: 'Stored conversation',
    availableCommands: [],
    configOptions: [],
    contextTokens: null,
    contextWindow: null,
    createdAt,
    cwd: '/project',
    id: 'conversation-id',
    modes: null,
    projectId: 'project-id',
    projectName: 'Project',
    providerId: 'cursor',
    providerSessionId: null,
    providerSessionPath: null,
    status: 'idle',
    transcript: [
      {
        content: 'Hello',
        createdAt,
        id: 'user',
        kind: 'message',
        role: 'user',
        streaming: false,
      },
    ],
    updatedAt: createdAt,
  };
}
