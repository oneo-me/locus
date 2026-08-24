import { type PointerEvent as ReactPointerEvent, useRef } from 'react';
import { useLocation, useNavigate } from 'react-router';
import type { WorkspaceView } from './shell';

interface WorkspaceMobileNavigationState {
  mobilePane?: 'primary' | 'secondary';
}

export const workspaceMobileSecondaryState = {
  mobilePane: 'secondary',
} satisfies WorkspaceMobileNavigationState;

export const workspaceMobilePrimaryState = {
  mobilePane: 'primary',
} satisfies WorkspaceMobileNavigationState;

function mobileNavigationState(value: unknown): WorkspaceMobileNavigationState {
  return value !== null && typeof value === 'object'
    ? (value as WorkspaceMobileNavigationState)
    : {};
}

export function workspaceShowsMobileContent(
  activeView: WorkspaceView,
  locationState: unknown,
): boolean {
  const pane = mobileNavigationState(locationState).mobilePane;
  if (pane) return pane === 'secondary';
  return activeView === 'documents';
}

export function workspaceRequestsMobileNavigation(
  locationState: unknown,
): boolean {
  return mobileNavigationState(locationState).mobilePane === 'primary';
}

export function workspaceMobileSwipeOpensNavigation(
  deltaX: number,
  deltaY: number,
): boolean {
  return deltaX >= 48 && Math.abs(deltaY) <= deltaX;
}

interface WorkspaceMobileSwipeStart {
  pointerId: number;
  x: number;
  y: number;
}

export function WorkspaceMobileEdgeSwipe() {
  const location = useLocation();
  const navigate = useNavigate();
  const swipeStart = useRef<WorkspaceMobileSwipeStart | null>(null);

  const openNavigation = () => {
    const state = mobileNavigationState(location.state);
    void navigate(`${location.pathname}${location.search}${location.hash}`, {
      replace: true,
      state: { ...state, mobilePane: 'primary' },
      preventScrollReset: true,
    });
  };

  const clearSwipe = (event: ReactPointerEvent<HTMLButtonElement>) => {
    if (swipeStart.current?.pointerId !== event.pointerId) return;
    swipeStart.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  };

  return (
    <button
      aria-label="展开文档导航"
      className="fixed inset-y-0 left-0 z-20 w-2.5 touch-pan-y border-0 bg-transparent p-0 outline-none focus-visible:bg-primary/10 focus-visible:ring-2 focus-visible:ring-ring md:hidden"
      data-workspace-mobile-edge-swipe
      onClick={openNavigation}
      onPointerCancel={clearSwipe}
      onPointerDown={(event) => {
        if (!event.isPrimary) return;
        swipeStart.current = {
          pointerId: event.pointerId,
          x: event.clientX,
          y: event.clientY,
        };
        event.currentTarget.setPointerCapture(event.pointerId);
      }}
      onPointerMove={(event) => {
        const start = swipeStart.current;
        if (!start || start.pointerId !== event.pointerId) return;
        const deltaX = event.clientX - start.x;
        const deltaY = event.clientY - start.y;
        if (deltaX < 0 || Math.abs(deltaY) > Math.max(16, deltaX)) {
          clearSwipe(event);
          return;
        }
        if (!workspaceMobileSwipeOpensNavigation(deltaX, deltaY)) return;
        event.preventDefault();
        clearSwipe(event);
        openNavigation();
      }}
      onPointerUp={clearSwipe}
      type="button"
    />
  );
}
