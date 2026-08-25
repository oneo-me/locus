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
    route('agents', 'routes/agents.tsx'),
    route('graph', 'routes/graph.tsx'),
    route('design-system', 'routes/design-system.tsx'),
    route('settings', 'routes/settings.tsx'),
  ]),
] satisfies RouteConfig;
