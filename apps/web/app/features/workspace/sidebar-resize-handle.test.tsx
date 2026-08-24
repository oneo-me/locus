import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, test } from 'vitest';
import { WorkspaceSidebarResizeHandle } from './sidebar-resize-handle';

describe('WorkspaceSidebarResizeHandle', () => {
  test('exposes its persisted width and keyboard-adjustable limits', () => {
    const markup = renderToStaticMarkup(
      <WorkspaceSidebarResizeHandle
        onWidthChange={() => {}}
        onWidthCommit={() => {}}
        width={312}
      />,
    );

    expect(markup).toContain('aria-label="调整文档侧栏宽度"');
    expect(markup).toContain('aria-orientation="vertical"');
    expect(markup).toContain('aria-valuemin="284"');
    expect(markup).toContain('aria-valuemax="420"');
    expect(markup).toContain('aria-valuenow="312"');
    expect(markup).toContain('tabindex="0"');
    expect(markup).toContain('md:block');
    expect(markup).not.toContain('hover:after:bg-primary');
    expect(markup).toContain('focus-visible:after:bg-primary');
  });
});
