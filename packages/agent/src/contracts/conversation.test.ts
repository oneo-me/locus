import assert from 'node:assert/strict';
import test from 'node:test';
import {
  agentConversationCanDelete,
  agentConversationDisplayTitle,
  agentConversationTitleFromPrompt,
  DEFAULT_AGENT_CONVERSATION_TITLE,
} from './conversation.js';

void test('only blocks deletion while a conversation is active', () => {
  assert.equal(agentConversationCanDelete('connecting'), false);
  assert.equal(agentConversationCanDelete('working'), false);
  assert.equal(agentConversationCanDelete('idle'), true);
  assert.equal(agentConversationCanDelete('waiting'), true);
  assert.equal(agentConversationCanDelete('failed'), true);
});

void test('uses the provider title when available', () => {
  assert.equal(
    agentConversationDisplayTitle({
      autoTitle: '修复 Tasks 侧栏',
    }),
    '修复 Tasks 侧栏',
  );
});

void test('uses the first seven words and caps prompt fallback titles', () => {
  assert.equal(
    agentConversationTitleFromPrompt('one two three four five six seven eight'),
    'one two three four five six seven',
  );
  assert.equal(
    agentConversationTitleFromPrompt('任'.repeat(60)),
    `${'任'.repeat(53)}…`,
  );
});

void test('empty automatic titles fall back to the default title', () => {
  assert.equal(
    agentConversationDisplayTitle({
      autoTitle: '  ',
    }),
    DEFAULT_AGENT_CONVERSATION_TITLE,
  );
});
