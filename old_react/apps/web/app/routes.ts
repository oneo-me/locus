import {
  type RouteConfig,
  index,
  layout,
  route,
} from '@react-router/dev/routes';

export default [
  layout('routes/app.tsx', [
    index('routes/app-index.tsx'),
    route('notes', 'routes/workspace.tsx', [
      index('routes/workspace-index.tsx'),
      route('*', 'routes/workspace-document.tsx'),
    ]),
    route('tasks/:taskId?', 'routes/tasks.tsx'),
    route('graph', 'routes/graph.tsx'),
    route('design-system', 'routes/design-system.tsx'),
    route('settings', 'routes/settings.tsx', [
      index('routes/settings-index.tsx'),
      route('general', 'routes/settings-general.tsx'),
      route('library', 'routes/settings-library.tsx'),
      route('terminal', 'routes/settings-terminal.tsx'),
      route('agents', 'routes/settings-agents.tsx'),
      route('archives', 'routes/settings-archives.tsx'),
      route('mcp', 'routes/settings-mcp.tsx'),
      route('about', 'routes/settings-about.tsx'),
    ]),
  ]),
] satisfies RouteConfig;
