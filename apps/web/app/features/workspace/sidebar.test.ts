import { describe, expect, test } from 'vitest';
import type { DocumentTreeNode } from '../library/tree';
import {
  centerTreeScrollTarget,
  focusSelectedNode,
  reconcileTreeItems,
  treeItemTapBehavior,
} from './sidebar';

function node(path: string, title: string): DocumentTreeNode {
  return {
    name: path,
    path,
    document: {
      attachmentCount: 0,
      path: `${path}/index.md`,
      directory: path,
      title,
    } as DocumentTreeNode['document'],
    children: [],
  };
}

describe('treeItemTapBehavior', () => {
  test('expands a collapsed node with children on mobile', () => {
    expect(treeItemTapBehavior(true, true, false)).toBe('expand');
  });

  test('views an open node with children on mobile', () => {
    expect(treeItemTapBehavior(true, true, true)).toBe('view');
  });

  test('views a leaf node on mobile', () => {
    expect(treeItemTapBehavior(true, false, false)).toBe('view');
  });

  test('always views on desktop regardless of children', () => {
    expect(treeItemTapBehavior(false, true, false)).toBe('view');
    expect(treeItemTapBehavior(false, true, true)).toBe('view');
    expect(treeItemTapBehavior(false, false, false)).toBe('view');
  });
});

describe('reconcileTreeItems', () => {
  test('preserves a local order while replacing stale node data', () => {
    const previous = [node('a', '旧 A'), node('b', '旧 B')];
    const latest = [node('b', '新 B'), node('a', '新 A')];

    expect(
      reconcileTreeItems(previous, latest).map((item) => item.document?.title),
    ).toEqual(['新 A', '新 B']);
  });

  test('removes deleted nodes and appends new nodes', () => {
    const previous = [node('removed', '旧笔记'), node('kept', '保留')];
    const latest = [node('kept', '保留'), node('added', '新增')];

    expect(
      reconcileTreeItems(previous, latest).map((item) => item.path),
    ).toEqual(['kept', 'added']);
  });
});

describe('focusSelectedNode', () => {
  test('keeps search paths alongside the selected branch', () => {
    const next = focusSelectedNode(
      ['projects', 'projects/asterweft'],
      ['health'],
    );

    expect(next).toEqual(new Set(['projects', 'projects/asterweft', 'health']));
  });

  test('keeps the selected path without search paths', () => {
    const next = focusSelectedNode(['guide', 'guide/setup'], []);

    expect(next).toEqual(new Set(['guide', 'guide/setup']));
  });

  test('starts fresh without the previously open sibling branch', () => {
    const next = focusSelectedNode(['projects'], []);

    expect(next).toEqual(new Set(['projects']));
  });
});

function scrollableNav(viewHeight: number): HTMLElement {
  const nav = {
    scrollTop: 0,
    getClientRects: () => [{ height: viewHeight }] as unknown as DOMRectList,
    getBoundingClientRect: () => ({ top: 100, height: viewHeight }),
  } as unknown as HTMLElement;
  return nav;
}

function selectedItem(top: number, height: number): HTMLElement {
  const target = {
    getClientRects: () => [{ height }] as unknown as DOMRectList,
    getBoundingClientRect: () => ({ top, height }),
  } as unknown as HTMLElement;
  return target;
}

describe('centerTreeScrollTarget', () => {
  test('scrolls a selected item below the viewport to the middle', () => {
    const nav = scrollableNav(200);
    nav.scrollTop = 0;
    const target = selectedItem(800, 32);

    centerTreeScrollTarget(nav, target);

    expect(nav.scrollTop).toBe(800 - 100 + 16 - 100);
  });

  test('scrolls a selected item above the viewport to the middle', () => {
    const nav = scrollableNav(200);
    nav.scrollTop = 800;
    const target = selectedItem(40, 32);

    centerTreeScrollTarget(nav, target);

    expect(nav.scrollTop).toBe(800 + (40 - 100 + 16 - 100));
  });

  test('keeps scroll position when the selected item is already centered', () => {
    const nav = scrollableNav(200);
    nav.scrollTop = 500;
    const target = selectedItem(184, 32);

    centerTreeScrollTarget(nav, target);

    expect(nav.scrollTop).toBe(500);
  });

  test('does nothing when the nav is not visible', () => {
    const nav = scrollableNav(200);
    Object.defineProperty(nav, 'getClientRects', {
      value: () => [] as unknown as DOMRectList,
    });
    nav.scrollTop = 120;
    const target = selectedItem(800, 32);

    centerTreeScrollTarget(nav, target);

    expect(nav.scrollTop).toBe(120);
  });
});
