import { describe, expect, test } from 'vitest';
import {
  GRAPH_ATTACHMENT_LINK_DISTANCE,
  GRAPH_FORCE_DEFAULTS,
  GRAPH_HIERARCHY_LINK_DISTANCE,
  graphNodeDiameter,
  graphNodeScreenRadius,
  initialGraphPosition,
} from './layout';

describe('graph layout', () => {
  test('keeps the Obsidian global force defaults', () => {
    expect(GRAPH_FORCE_DEFAULTS).toEqual({
      centerStrength: 0.1,
      collisionRadius: 60,
      collisionStrength: 0.5,
      linkDistance: 250,
      repelStrength: -1_000,
    });
    expect(GRAPH_HIERARCHY_LINK_DISTANCE).toBe(160);
    expect(GRAPH_ATTACHMENT_LINK_DISTANCE).toBe(64);
  });

  test('keeps node diameters in the compact graph range', () => {
    expect(graphNodeDiameter(0)).toBe(12);
    expect(graphNodeDiameter(8)).toBe(15);
    expect(graphNodeDiameter(10_000)).toBe(44);
  });

  test('only scales node radii in distant overviews', () => {
    expect(graphNodeScreenRadius(12, 1)).toBe(6);
    expect(graphNodeScreenRadius(12, 0.5)).toBe(6);
    expect(graphNodeScreenRadius(12, 0.4)).toBe(6);
    expect(graphNodeScreenRadius(12, 0.125)).toBe(1.875);
    expect(graphNodeScreenRadius(12, 0.01)).toBe(0.75);
  });

  test('produces stable positions inside the area-scaled seed disk', () => {
    const node = { id: 'guide/index.md' };
    const first = initialGraphPosition(node, 100);
    const second = initialGraphPosition(node, 100);
    const radius = Math.hypot(first.x, first.y);
    const outerRadius = Math.sqrt((100 * 60 * 60) / Math.PI);

    expect(second).toEqual(first);
    expect(radius).toBeLessThanOrEqual(outerRadius);
  });

  test('seeds same-directory nodes into the same cluster neighbourhood', () => {
    const left = initialGraphPosition({ id: 'projects/a/index.md' }, 80, {
      index: 0,
      count: 2,
    });
    const right = initialGraphPosition({ id: 'projects/b/index.md' }, 80, {
      index: 0,
      count: 2,
    });
    const other = initialGraphPosition({ id: 'journal/a/index.md' }, 80, {
      index: 1,
      count: 2,
    });

    const leftToRight = Math.hypot(left.x - right.x, left.y - right.y);
    const leftToOther = Math.hypot(left.x - other.x, left.y - other.y);

    expect(leftToRight).toBeLessThan(leftToOther);
  });
});
