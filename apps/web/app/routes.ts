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
    route('*', 'routes/workspace-document.tsx'),
  ]),
] satisfies RouteConfig;
