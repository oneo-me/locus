import { matchRoutes } from 'react-router';
import { describe, expect, test } from 'vitest';
import {
  resolveDocumentLoaderPathname,
  shouldRevalidate as shouldRevalidateDocument,
} from './workspace-document';
import { shouldRevalidate as shouldRevalidateGraph } from './workspace-graph';
import { shouldRevalidate as shouldRevalidateWorkspace } from './workspace';

const routeTree = [
  { id: 'design-system', path: '/design-system' },
  {
    id: 'workspace',
    path: '/',
    children: [
      { id: 'document-index', index: true },
      { id: 'graph', path: 'graph' },
      { id: 'document', path: '*' },
    ],
  },
];

function matchedRouteIds(pathname: string): Array<string | undefined> {
  return matchRoutes(routeTree, pathname)?.map((match) => match.route.id) ?? [];
}

function revalidationArgs(current: string, next: string) {
  return {
    currentUrl: new URL(current, 'http://asterweft.test'),
    nextUrl: new URL(next, 'http://asterweft.test'),
    defaultShouldRevalidate: true,
  } as Parameters<typeof shouldRevalidateWorkspace>[0];
}

describe('React Router workspace routes', () => {
  test('keeps the design system outside the workspace content routes', () => {
    expect(matchedRouteIds('/')).toEqual(['workspace', 'document-index']);
    expect(matchedRouteIds('/graph')).toEqual(['workspace', 'graph']);
    expect(matchedRouteIds('/design-system')).toEqual(['design-system']);
    expect(matchedRouteIds('/guide')).toEqual(['workspace', 'document']);
  });

  test('resolves document data requests inside the document route', () => {
    expect(resolveDocumentLoaderPathname('/guide.data')).toBe('/guide');
    expect(resolveDocumentLoaderPathname('/guide/_.data')).toBe('/guide/');
  });

  test('revalidates workspace data for search but not article path changes', () => {
    expect(
      shouldRevalidateWorkspace(revalidationArgs('/guide', '/canvas')),
    ).toBe(false);
    expect(
      shouldRevalidateWorkspace(revalidationArgs('/guide', '/guide?q=canvas')),
    ).toBe(true);
  });

  test('revalidates document data for paths but not search changes', () => {
    expect(
      shouldRevalidateDocument(revalidationArgs('/guide', '/canvas')),
    ).toBe(true);
    expect(
      shouldRevalidateDocument(revalidationArgs('/guide', '/guide?q=canvas')),
    ).toBe(false);
  });

  test('does not revalidate graph data for workspace searches', () => {
    expect(
      shouldRevalidateGraph(revalidationArgs('/graph', '/graph?q=canvas')),
    ).toBe(false);
  });

  test('keeps explicit same-URL and submission revalidation working', () => {
    expect(
      shouldRevalidateWorkspace(revalidationArgs('/guide', '/guide')),
    ).toBe(true);
    expect(shouldRevalidateDocument(revalidationArgs('/guide', '/guide'))).toBe(
      true,
    );
    expect(
      shouldRevalidateWorkspace({
        ...revalidationArgs('/guide', '/guide'),
        formMethod: 'POST',
      }),
    ).toBe(true);
  });
});
