import { TasksShell } from '~/features/tasks/shell';
import { taskApi } from '~/lib/task-api';
import type { Route } from './+types/tasks';

export async function clientLoader({ params }: Route.ClientLoaderArgs) {
  const [{ conversations }, { projects }, providers] = await Promise.all([
    taskApi.list(),
    taskApi.listProjects(),
    taskApi.providers(),
  ]);
  const conversation = params.taskId ? await taskApi.get(params.taskId) : null;
  return { conversation, conversations, projects, providers };
}

export default function TasksRoute({ loaderData }: Route.ComponentProps) {
  return (
    <TasksShell
      initialData={loaderData}
      key={loaderData.conversation?.id ?? 'new'}
    />
  );
}

export function meta() {
  return [{ title: 'Tasks — Locus' }];
}
