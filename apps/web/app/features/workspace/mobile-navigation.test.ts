import { describe, expect, test } from 'vitest';
import {
  workspaceMobileSwipeOpensNavigation,
  workspaceRequestsMobileNavigation,
  workspaceShowsMobileContent,
} from './mobile-navigation';

describe('workspace mobile navigation', () => {
  test('starts the graph route on the primary navigation pane', () => {
    expect(workspaceShowsMobileContent('graph', null)).toBe(false);
    expect(workspaceShowsMobileContent('graph', {})).toBe(false);
  });

  test('opens graph and document routes on the secondary pane', () => {
    expect(
      workspaceShowsMobileContent('graph', { mobilePane: 'secondary' }),
    ).toBe(true);
    expect(workspaceShowsMobileContent('documents', null)).toBe(true);
  });

  test('keeps the current route while explicitly showing the primary pane', () => {
    expect(
      workspaceShowsMobileContent('documents', { mobilePane: 'primary' }),
    ).toBe(false);
    expect(
      workspaceShowsMobileContent('graph', { mobilePane: 'primary' }),
    ).toBe(false);
    expect(workspaceRequestsMobileNavigation({ mobilePane: 'primary' })).toBe(
      true,
    );
  });

  test('opens navigation only after a deliberate inward edge swipe', () => {
    expect(workspaceMobileSwipeOpensNavigation(47, 0)).toBe(false);
    expect(workspaceMobileSwipeOpensNavigation(48, 16)).toBe(true);
    expect(workspaceMobileSwipeOpensNavigation(48, 49)).toBe(false);
    expect(workspaceMobileSwipeOpensNavigation(-48, 0)).toBe(false);
  });
});
