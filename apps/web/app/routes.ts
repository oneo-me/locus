import {
  type RouteConfig,
  index,
  layout,
  route,
} from '@react-router/dev/routes';

export default [
  route('design-system', 'routes/design-system.tsx'),
  layout('routes/workspace.tsx', [
    index('routes/workspace-index.tsx'),
    route('graph', 'routes/workspace-graph.tsx'),
    route('*', 'routes/workspace-document.tsx'),
  ]),
  route('api/library', 'routes/api.library.ts'),
  route('api/library/graph', 'routes/api.library.graph.ts'),
  route(
    'api/library/document/metadata',
    'routes/api.library.document.metadata.ts',
  ),
  route('api/library/document/icon', 'routes/api.library.document.icon.ts'),
  route(
    'api/library/graph-participation',
    'routes/api.library.graph-participation.ts',
  ),
  route('api/library/reorder', 'routes/api.library.reorder.ts'),
  route('api/settings', 'routes/api.settings.ts'),
  route('api/graph-events', 'routes/api.graph-events.ts'),
  route('api/assets', 'routes/api.assets.ts'),
  route('mcp', 'routes/mcp.ts'),
] satisfies RouteConfig;
