import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { MemoryRouter } from 'react-router';
import { describe, expect, test } from 'vitest';
import {
  getWorkspaceEventStatusText,
  WorkspaceBrandMark,
  WorkspaceHeader,
} from './header';

describe('workspace event status', () => {
  test('describes connecting and reconnecting states', () => {
    expect(
      getWorkspaceEventStatusText({
        connection: 'connecting',
        lastEventAt: null,
      }),
    ).toBe('正在连接实时事件…');
    expect(
      getWorkspaceEventStatusText({
        connection: 'reconnecting',
        lastEventAt: '14:03:21',
      }),
    ).toBe('连接中断，正在重试…');
  });

  test('shows only the latest event time when connected', () => {
    expect(
      getWorkspaceEventStatusText({
        connection: 'connected',
        lastEventAt: null,
      }),
    ).toBe('正在监听文档事件');
    expect(
      getWorkspaceEventStatusText({
        connection: 'connected',
        lastEventAt: '14:03:21',
      }),
    ).toBe('14:03:21');
  });

  test('shows document refreshes before the latest event time', () => {
    expect(
      getWorkspaceEventStatusText(
        {
          connection: 'connected',
          lastEventAt: '14:03:21',
        },
        true,
      ),
    ).toBe('正在刷新文档库…');
  });
});

describe('workspace brand mark', () => {
  test('uses the app icon and clips it to a circle', () => {
    const markup = renderToStaticMarkup(createElement(WorkspaceBrandMark));

    expect(markup).toContain('src="/icons/asterweft-mark-64.png"');
    expect(markup).toContain('overflow-hidden');
    expect(markup).toContain('rounded-full');
    expect(markup).not.toContain('rounded-lg');
    expect(markup).toContain('bg-card');
  });

  test('separates the product name from the quieter event status', () => {
    const markup = renderToStaticMarkup(
      createElement(
        MemoryRouter,
        null,
        createElement(WorkspaceHeader, {
          activeView: 'documents',
          eventStatus: {
            connection: 'connected',
            lastEventAt: '14:03:21',
          },
          refreshing: false,
        }),
      ),
    );

    expect(markup).toContain('text-sm leading-5 font-medium text-foreground');
    expect(markup).toContain(
      'text-xs leading-4 font-normal text-muted-foreground',
    );
    expect(markup).toContain('aria-label="知识图谱"');
    expect(markup).toContain('href="/graph"');
  });

  test('marks the graph action as the current page inside the brand header', () => {
    const markup = renderToStaticMarkup(
      createElement(
        MemoryRouter,
        null,
        createElement(WorkspaceHeader, {
          activeView: 'graph',
          eventStatus: {
            connection: 'connected',
            lastEventAt: null,
          },
          query: '项目',
          refreshing: false,
        }),
      ),
    );

    expect(markup).toContain('aria-current="page"');
    expect(markup).toContain('href="/graph?q=%E9%A1%B9%E7%9B%AE"');
    expect(markup).toContain('bg-sidebar-accent');
    expect(markup).not.toContain('border-primary');
    expect(markup).not.toContain('from-background');
  });

  test('announces an active document refresh', () => {
    const markup = renderToStaticMarkup(
      createElement(
        MemoryRouter,
        null,
        createElement(WorkspaceHeader, {
          activeView: 'documents',
          eventStatus: {
            connection: 'connected',
            lastEventAt: '14:03:21',
          },
          refreshing: true,
        }),
      ),
    );

    expect(markup).toContain('aria-busy="true"');
    expect(markup).toContain('正在刷新文档库…');
  });
});
