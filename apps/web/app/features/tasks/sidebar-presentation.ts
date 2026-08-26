import type { AgentConversationSummary } from '@locus/agent/contracts/conversation';

export type TaskDateGroup =
  | 'today'
  | 'yesterday'
  | 'week'
  | 'month'
  | 'year'
  | 'more';

export interface TaskConversationGroup {
  id: TaskDateGroup;
  label: string;
  conversations: AgentConversationSummary[];
}

const GROUPS: Array<{ id: TaskDateGroup; label: string }> = [
  { id: 'today', label: '今天' },
  { id: 'yesterday', label: '昨天' },
  { id: 'week', label: '本周' },
  { id: 'month', label: '本月' },
  { id: 'year', label: '今年' },
  { id: 'more', label: '更早' },
];

export function groupConversationsByDate(
  conversations: AgentConversationSummary[],
  now = new Date(),
): TaskConversationGroup[] {
  const groups = new Map<TaskDateGroup, AgentConversationSummary[]>();
  const sorted = [...conversations].sort((left, right) =>
    right.activityAt.localeCompare(left.activityAt),
  );
  for (const conversation of sorted) {
    const id = conversationDateGroup(new Date(conversation.activityAt), now);
    const items = groups.get(id) ?? [];
    items.push(conversation);
    groups.set(id, items);
  }
  return GROUPS.filter(({ id }) => groups.has(id)).map(({ id, label }) => ({
    conversations: groups.get(id) ?? [],
    id,
    label,
  }));
}

export function conversationDateGroup(
  conversationDate: Date,
  now = new Date(),
): TaskDateGroup {
  const today = localDateStart(now);
  const conversationDay = localDateStart(conversationDate);
  if (conversationDay >= today) return 'today';

  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);
  if (conversationDay.getTime() === yesterday.getTime()) return 'yesterday';

  const weekStart = new Date(today);
  weekStart.setDate(weekStart.getDate() - ((weekStart.getDay() + 6) % 7));
  if (conversationDay >= weekStart) return 'week';
  if (
    conversationDay.getFullYear() === today.getFullYear() &&
    conversationDay.getMonth() === today.getMonth()
  ) {
    return 'month';
  }
  if (conversationDay.getFullYear() === today.getFullYear()) return 'year';
  return 'more';
}

export function relativeTimeLabel(
  activityAt: string,
  now = new Date(),
): string {
  const seconds = Math.max(
    0,
    Math.floor((now.getTime() - new Date(activityAt).getTime()) / 1_000),
  );
  if (seconds < 60) return '刚刚';
  if (seconds < 3_600) return `${Math.floor(seconds / 60)} 分钟前`;
  if (seconds < 86_400) return `${Math.floor(seconds / 3_600)} 小时前`;
  return `${Math.floor(seconds / 86_400)} 天前`;
}

export function nextSidebarTimeUpdateDelay(
  conversations: AgentConversationSummary[],
  now = new Date(),
): number {
  const nowMilliseconds = now.getTime();
  const tomorrow = new Date(
    now.getFullYear(),
    now.getMonth(),
    now.getDate() + 1,
  );
  let delay = tomorrow.getTime() - nowMilliseconds;
  for (const conversation of conversations) {
    const elapsed = Math.max(
      0,
      Math.floor(
        (nowMilliseconds - new Date(conversation.activityAt).getTime()) / 1_000,
      ),
    );
    const step = elapsed < 3_600 ? 60 : elapsed < 86_400 ? 3_600 : 86_400;
    delay = Math.min(delay, (step - (elapsed % step)) * 1_000);
  }
  return Math.max(1_000, delay);
}

function localDateStart(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}
