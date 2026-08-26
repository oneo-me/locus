import assert from 'node:assert/strict';
import test from 'node:test';
import type { AgentConversationSummary } from '@locus/agent/contracts/conversation';
import {
  conversationDateGroup,
  groupConversationsByDate,
  relativeTimeLabel,
} from './sidebar-presentation';

void test('groups conversations by local calendar boundaries', () => {
  const now = new Date(2026, 7, 26, 12);
  assert.equal(conversationDateGroup(new Date(2026, 7, 26), now), 'today');
  assert.equal(conversationDateGroup(new Date(2026, 7, 25), now), 'yesterday');
  assert.equal(conversationDateGroup(new Date(2026, 7, 24), now), 'week');
  assert.equal(conversationDateGroup(new Date(2026, 7, 1), now), 'month');
  assert.equal(conversationDateGroup(new Date(2026, 0, 1), now), 'year');
  assert.equal(conversationDateGroup(new Date(2025, 11, 31), now), 'more');
});

void test('keeps time groups and conversations in newest-first order', () => {
  const conversations = [
    conversation('older', new Date(2026, 7, 26, 9)),
    conversation('yesterday', new Date(2026, 7, 25, 18)),
    conversation('newer', new Date(2026, 7, 26, 11)),
  ];
  const groups = groupConversationsByDate(
    conversations,
    new Date(2026, 7, 26, 12),
  );
  assert.deepEqual(
    groups.map((group) => ({
      ids: group.conversations.map(({ id }) => id),
      label: group.label,
    })),
    [
      { ids: ['newer', 'older'], label: '今天' },
      { ids: ['yesterday'], label: '昨天' },
    ],
  );
});

void test('does not create an empty today group', () => {
  assert.deepEqual(groupConversationsByDate([], new Date(2026, 7, 26, 12)), []);
});

void test('formats relative activity time with Waku units', () => {
  const now = new Date('2026-08-26T12:00:00.000Z');
  assert.equal(relativeTimeLabel('2026-08-26T11:59:01.000Z', now), '刚刚');
  assert.equal(relativeTimeLabel('2026-08-26T11:55:00.000Z', now), '5 分钟前');
  assert.equal(relativeTimeLabel('2026-08-26T10:00:00.000Z', now), '2 小时前');
  assert.equal(relativeTimeLabel('2026-08-19T12:00:00.000Z', now), '7 天前');
});

function conversation(id: string, activityAt: Date): AgentConversationSummary {
  return {
    activityAt: activityAt.toISOString(),
    autoTitle: id,
    createdAt: activityAt.toISOString(),
    id,
    projectId: 'project',
    projectName: 'Project',
    providerId: 'cursor',
    status: 'idle',
    title: '新任务',
    updatedAt: activityAt.toISOString(),
  };
}
