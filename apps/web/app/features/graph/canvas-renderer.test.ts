import { describe, expect, test } from 'vitest';
import {
  buildGraphAdjacency,
  compensateCameraForGraphCenter,
  computeHoverDepths,
  fitGraphLabel,
  graphBoundsCenter,
  graphRevealNodeCount,
  graphWheelInteraction,
  HOVER_NODE_EMPHASIS,
  hoverEmphasisForDepth,
  hoverEmphasisForEdge,
  hoverMuteForEdge,
  hoverMuteForNode,
  mixGraphColor,
} from './canvas-renderer';

describe('graph canvas presentation', () => {
  test('reveals a large graph progressively, then includes every node', () => {
    expect(graphRevealNodeCount(0, 0)).toBe(0);
    expect(graphRevealNodeCount(100, -1)).toBe(0);
    expect(graphRevealNodeCount(100, 0)).toBe(1);
    expect(graphRevealNodeCount(100, 130)).toBe(75);
    expect(graphRevealNodeCount(100, 260)).toBe(100);
    expect(graphRevealNodeCount(100, 0, 0)).toBe(100);
  });

  test('keeps the graph center fixed while initialization positions settle', () => {
    const previousCenter = graphBoundsCenter(
      new Float32Array([-100, -40, 60, 80]),
    );
    const nextCenter = graphBoundsCenter(new Float32Array([-140, -80, 40, 60]));
    expect(previousCenter).toEqual({ x: -20, y: 20 });
    expect(nextCenter).toEqual({ x: -50, y: -10 });
    if (!previousCenter || !nextCenter) {
      throw new Error('Expected graph bounds centers.');
    }

    const camera = { x: 500, y: 300, zoom: 0.75 };
    const compensated = compensateCameraForGraphCenter(
      camera,
      previousCenter,
      nextCenter,
    );

    expect(previousCenter.x * camera.zoom + camera.x).toBeCloseTo(
      nextCenter.x * compensated.zoom + compensated.x,
    );
    expect(previousCenter.y * camera.zoom + camera.y).toBeCloseTo(
      nextCenter.y * compensated.zoom + compensated.y,
    );
  });

  test('maps ordinary wheel input to zooming without guessing the device', () => {
    expect(
      graphWheelInteraction(
        {
          ctrlKey: false,
          deltaMode: 0,
          deltaY: -12,
        },
        800,
      ),
    ).toEqual({ delta: -12, source: 'wheel' });
    expect(
      graphWheelInteraction(
        {
          ctrlKey: false,
          deltaMode: 1,
          deltaY: 3,
        },
        800,
      ),
    ).toEqual({ delta: 48, source: 'wheel' });
  });

  test('maps a trackpad pinch to pinch zooming', () => {
    expect(
      graphWheelInteraction(
        {
          ctrlKey: true,
          deltaMode: 0,
          deltaY: -8,
        },
        800,
      ),
    ).toEqual({ delta: -8, source: 'pinch' });
  });

  test('normalizes page-mode pinch deltas', () => {
    expect(
      graphWheelInteraction(
        {
          ctrlKey: true,
          deltaMode: 2,
          deltaY: 1,
        },
        800,
      ),
    ).toEqual({ delta: 800, source: 'wheel' });
  });

  test('interpolates highlight colors without changing geometry', () => {
    expect(mixGraphColor('#000000', '#ffffff', 0)).toBe('rgb(0, 0, 0)');
    expect(mixGraphColor('#000000', '#ffffff', 0.5)).toBe('rgb(128, 128, 128)');
    expect(mixGraphColor('#000000', '#ffffff', 1)).toBe('rgb(255, 255, 255)');
    expect(mixGraphColor('hsl(120, 100%, 25%)', '#ffffff', 0)).toBe(
      'rgb(0, 128, 0)',
    );
    expect(mixGraphColor('hsl(120, 100%, 25%)', '#ffffff', 0.5)).toBe(
      'rgb(128, 192, 128)',
    );
  });

  test('uses an ellipsis instead of horizontally compressing labels', () => {
    const measure = (value: string) => Array.from(value).length * 10;

    expect(fitGraphLabel('short', 60, measure)).toBe('short');
    expect(fitGraphLabel('a very long label', 60, measure)).toBe('a ver…');
  });

  test('highlights only the hovered node and its direct neighbors', () => {
    // 0-1-2-3-4，以及 1-5 旁支
    const adjacency = buildGraphAdjacency(6, [
      { source: 0, target: 1 },
      { source: 1, target: 2 },
      { source: 2, target: 3 },
      { source: 3, target: 4 },
      { source: 1, target: 5 },
    ]);
    const depths = computeHoverDepths(adjacency, 0);

    expect(Object.fromEntries(depths)).toEqual({
      0: 0,
      1: 1,
    });
    expect(depths.has(2)).toBe(false);
    expect(depths.has(4)).toBe(false);
    expect(depths.has(5)).toBe(false);
    expect(hoverEmphasisForDepth(0)).toBe(HOVER_NODE_EMPHASIS[0]);
    expect(hoverEmphasisForDepth(1)).toBe(HOVER_NODE_EMPHASIS[1]);
    expect(hoverEmphasisForDepth(undefined)).toBe(0);
  });

  test('scores hover edges by the farther endpoint depth', () => {
    const depths = new Map([
      [0, 0],
      [1, 1],
      [2, 2],
      [3, 3],
    ]);

    expect(hoverEmphasisForEdge(depths, 0, 1)).toBe(HOVER_NODE_EMPHASIS[1]);
    expect(hoverEmphasisForEdge(depths, 1, 2)).toBe(0);
    expect(hoverEmphasisForEdge(depths, 2, 3)).toBe(0);
    expect(hoverEmphasisForEdge(depths, 3, 4)).toBe(0);
    expect(hoverEmphasisForEdge(null, 0, 1)).toBe(0);
  });

  test('grays only nodes outside the hovered relationship neighborhood', () => {
    const previousDepths = new Map([
      [0, 0],
      [1, 1],
    ]);
    const currentDepths = new Map([
      [2, 0],
      [3, 1],
    ]);

    expect(hoverMuteForNode(null, currentDepths, 2, 1)).toBe(0);
    expect(hoverMuteForNode(null, currentDepths, 0, 1)).toBe(1);
    expect(hoverMuteForNode(previousDepths, currentDepths, 0, 0.5)).toBe(0.5);
    expect(hoverMuteForNode(previousDepths, null, 4, 0.5)).toBe(0.5);
    expect(hoverMuteForNode(previousDepths, currentDepths, 4, 0.5)).toBe(1);
  });

  test('grays only edges outside the hovered relationship neighborhood', () => {
    const previousDepths = new Map([
      [0, 0],
      [1, 1],
    ]);
    const currentDepths = new Map([
      [2, 0],
      [3, 1],
    ]);

    expect(hoverMuteForEdge(null, currentDepths, 2, 3, 1)).toBe(0);
    expect(hoverMuteForEdge(null, currentDepths, 1, 2, 1)).toBe(1);
    expect(hoverMuteForEdge(previousDepths, currentDepths, 0, 1, 0.5)).toBe(
      0.5,
    );
    expect(hoverMuteForEdge(previousDepths, null, 0, 4, 0.5)).toBe(0.5);
    expect(hoverMuteForEdge(previousDepths, currentDepths, 0, 4, 0.5)).toBe(1);
  });
});
