import { describe, expect, test } from 'vitest';
import {
  graphLayoutOutputInterval,
  positionNewGraphNode,
  resolveGraphLayoutLinks,
} from './layout-worker';

describe('incremental graph layout', () => {
  test('reduces layout output frequency as graph complexity grows', () => {
    expect(graphLayoutOutputInterval(100, 200)).toBeCloseTo(1_000 / 60);
    expect(graphLayoutOutputInterval(600, 600)).toBeCloseTo(1_000 / 30);
    expect(graphLayoutOutputInterval(2_000, 2_000)).toBeCloseTo(1_000 / 20);
  });

  test('places a connected node near its existing neighbours', () => {
    const position = positionNewGraphNode(
      { id: 'new', x: 0, y: 10 },
      [
        { source: 'left', target: 'new' },
        { source: 'new', target: 'right' },
      ],
      new Map([
        ['left', { x: 80, y: 100 }],
        ['right', { x: 120, y: 100 }],
      ]),
    );

    expect(position.x).toBeCloseTo(100);
    expect(position.y).toBeCloseTo(124);
  });

  test('keeps the deterministic seed for an isolated node', () => {
    expect(
      positionNewGraphNode({ id: 'new', x: -20, y: 30 }, [], new Map()),
    ).toEqual({ x: -20, y: 30 });
  });

  test('resolves link strength from the weaker endpoint degree', () => {
    const links = resolveGraphLayoutLinks([
      { source: 'root', target: 'a', distance: 160 },
      { source: 'root', target: 'b', distance: 160 },
      { source: 'root', target: 'c', distance: 160 },
      { source: 'a', target: 'leaf', distance: 160 },
    ]);

    // degrees: root=3, a=2, b=1, c=1, leaf=1
    expect(links.map((link) => link.strength)).toEqual([0.5, 1, 1, 1]);
    expect(links.every((link) => link.distance === 160)).toBe(true);
  });

  test('weakens busy endpoints so dense parents do not collapse children', () => {
    const links = resolveGraphLayoutLinks([
      { source: 'hub', target: 'a' },
      { source: 'hub', target: 'b' },
      { source: 'hub', target: 'c' },
      { source: 'hub', target: 'd' },
      { source: 'a', target: 'b' },
    ]);

    // degrees: hub=4, a=2, b=2, c=1, d=1
    expect(links[0]?.strength).toBe(0.5);
    expect(links[2]?.strength).toBe(1);
    expect(links[4]?.strength).toBe(0.5);
  });
});
