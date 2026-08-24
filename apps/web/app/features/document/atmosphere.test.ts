import { describe, expect, test } from 'vitest';
import {
  deriveDocumentAtmosphere,
  deriveYamlPanelBackground,
  deriveYamlPanelHue,
} from './atmosphere';

describe('deriveDocumentAtmosphere', () => {
  test('returns identical output for the same title and theme', () => {
    const first = deriveDocumentAtmosphere('架构概览', 'light');
    const second = deriveDocumentAtmosphere('架构概览', 'light');

    expect(first).toEqual(second);
    expect(deriveDocumentAtmosphere('架构概览', 'dark')).toEqual(
      deriveDocumentAtmosphere('架构概览', 'dark'),
    );
  });

  test('treats title text as an exact seed without case folding', () => {
    const lower = deriveDocumentAtmosphere('Setup Guide', 'light');
    const upper = deriveDocumentAtmosphere('setup guide', 'light');

    expect(lower).not.toEqual(upper);
  });

  test('produces different parameters for different titles', () => {
    const a = deriveDocumentAtmosphere('安装指南', 'light');
    const b = deriveDocumentAtmosphere('部署手册', 'light');

    expect(a).not.toEqual(b);
  });

  test('keeps a stable fallback for an empty title', () => {
    expect(deriveDocumentAtmosphere('', 'light')).toEqual(
      deriveDocumentAtmosphere('', 'light'),
    );
    expect(
      deriveDocumentAtmosphere('', 'light').shapes.length,
    ).toBeGreaterThanOrEqual(4);
  });

  test('stays within restrained visual ranges', () => {
    const atmosphere = deriveDocumentAtmosphere('阅读体验', 'light');

    expect(atmosphere.hue).toBeGreaterThanOrEqual(190);
    expect(atmosphere.hue).toBeLessThanOrEqual(250);
    expect(atmosphere.shapes.length).toBeGreaterThanOrEqual(4);
    expect(atmosphere.shapes.length).toBeLessThanOrEqual(6);
    expect([
      'none',
      'grid',
      'lines',
      'dots',
      'diagonal',
      'diamonds',
      'waves',
    ]).toContain(atmosphere.backdrop.kind);

    for (const { shape, motion } of atmosphere.shapes) {
      expect(shape.opacity).toBeGreaterThan(0);
      expect(shape.opacity).toBeLessThanOrEqual(0.22);
      expect(motion.duration).toBeGreaterThan(10);
      expect(['float', 'spin']).toContain(motion.mode);
    }
  });

  test('derives a stable translucent yaml panel tint from the title hue', () => {
    const hue = deriveYamlPanelHue('入门指南');
    const background = deriveYamlPanelBackground('入门指南', 'light');

    expect(hue).toEqual(deriveYamlPanelHue('入门指南'));
    expect(background).toEqual(deriveYamlPanelBackground('入门指南', 'light'));
    expect(background).toContain(String(hue));
    expect(background).toMatch(/^hsl\([\d.]+ [\d.]+% [\d.]+% \/ 0\.48\)$/);
    expect(hue).not.toEqual(deriveYamlPanelHue('部署手册'));
  });

  test('can derive backdrop and shapes independently across titles', () => {
    const samples = [
      '网格笔记',
      '线条草稿',
      '圆点索引',
      '斜线备忘',
      '空白封面',
      '几何练习',
      '结构说明',
      '视觉试验',
    ].map((title) => deriveDocumentAtmosphere(title, 'light'));

    expect(
      new Set(samples.map((sample) => sample.backdrop.kind)).size,
    ).toBeGreaterThan(1);
    expect(
      new Set(
        samples.flatMap((sample) =>
          sample.shapes.map((instance) => instance.shape.kind),
        ),
      ).size,
    ).toBeGreaterThan(1);
  });

  test('varies backdrop tiles while keeping every pattern regular and restrained', () => {
    const samples = Array.from({ length: 512 }, (_, index) =>
      deriveDocumentAtmosphere(`背景样本 ${index}`, 'light'),
    );

    expect(new Set(samples.map(({ backdrop }) => backdrop.kind))).toEqual(
      new Set([
        'none',
        'grid',
        'lines',
        'dots',
        'diagonal',
        'diamonds',
        'waves',
      ]),
    );

    for (const { backdrop } of samples) {
      if (backdrop.kind === 'none') continue;

      expect(backdrop.spacing).toBeGreaterThanOrEqual(18);
      expect(backdrop.spacing).toBeLessThanOrEqual(43);
      expect(backdrop.offsetX).toBeGreaterThanOrEqual(0);
      expect(backdrop.offsetX).toBeLessThan(backdrop.spacing);
      expect(backdrop.offsetY).toBeGreaterThanOrEqual(0);
      expect(backdrop.offsetY).toBeLessThan(backdrop.spacing);
      expect(backdrop.opacity).toBeGreaterThan(0);
      expect(backdrop.opacity).toBeLessThanOrEqual(0.1);

      if (backdrop.kind === 'dots') {
        expect(backdrop.radius).toBeGreaterThanOrEqual(0.55);
        expect(backdrop.radius).toBeLessThanOrEqual(1.1);
      } else {
        expect(backdrop.strokeWidth).toBeGreaterThanOrEqual(0.45);
        expect(backdrop.strokeWidth).toBeLessThanOrEqual(0.95);
      }

      if (backdrop.kind === 'waves') {
        expect(backdrop.amplitude).toBeGreaterThanOrEqual(2.4);
        expect(backdrop.amplitude).toBeLessThan(backdrop.spacing / 4);
      }
    }
  });

  test('gives non-empty backdrops an independent, slow, visible drift', () => {
    const samples = Array.from({ length: 256 }, (_, index) =>
      deriveDocumentAtmosphere(`背景动画 ${index}`, 'light'),
    );

    for (const { backdrop, backdropMotion } of samples) {
      if (backdrop.kind === 'none') {
        expect(backdropMotion).toBeNull();
        continue;
      }

      expect(backdropMotion).not.toBeNull();
      if (!backdropMotion) continue;

      expect(backdropMotion.duration).toBeGreaterThanOrEqual(140);
      expect(backdropMotion.duration).toBeLessThanOrEqual(220);
      expect(backdropMotion.delay).toBeGreaterThanOrEqual(-120);
      expect(backdropMotion.delay).toBeLessThanOrEqual(0);
      expect(
        Math.hypot(backdropMotion.driftX, backdropMotion.driftY),
      ).toBeGreaterThanOrEqual(7.5);
      expect(Math.abs(backdropMotion.driftX)).toBeLessThanOrEqual(22);
      expect(Math.abs(backdropMotion.driftY)).toBeLessThanOrEqual(12.76);
      expect(Math.abs(backdropMotion.swayX)).toBeLessThanOrEqual(18.92);
      expect(Math.abs(backdropMotion.swayY)).toBeLessThanOrEqual(10.98);
    }
  });

  test('uses a coherent shape vocabulary with a clear first-three hierarchy', () => {
    const samples = [
      '架构概览',
      '入门指南',
      'Setup Guide',
      '视觉试验',
      '部署手册',
      '阅读体验',
    ].map((title) => deriveDocumentAtmosphere(title, 'light'));

    for (const atmosphere of samples) {
      expect(
        new Set(
          atmosphere.shapes.slice(0, 3).map((instance) => instance.shape.kind),
        ).size,
      ).toBe(3);
    }
  });

  test('varies layout, count, and shape vocabulary across titles', () => {
    const samples = Array.from({ length: 256 }, (_, index) =>
      deriveDocumentAtmosphere(`随机文档 ${index}`, 'light'),
    );
    const signatures = new Set(
      samples.map((atmosphere) =>
        atmosphere.shapes.map(({ shape }) => shape.kind).join(','),
      ),
    );
    const anchorSignatures = new Set(
      samples.map(({ shapes }) => {
        const shape = shapes[0]?.shape;
        if (!shape) return 'missing';
        if (shape.kind === 'roundedRect') {
          return `${Math.round((shape.x + shape.width / 2) / 16)}:${Math.round((shape.y + shape.height / 2) / 16)}`;
        }
        if (shape.kind === 'triangle') {
          const coordinates = shape.points
            .match(/-?\d+(?:\.\d+)?/g)
            ?.map(Number);
          const x =
            ((coordinates?.[0] ?? 0) +
              (coordinates?.[2] ?? 0) +
              (coordinates?.[4] ?? 0)) /
            3;
          const y =
            ((coordinates?.[1] ?? 0) +
              (coordinates?.[3] ?? 0) +
              (coordinates?.[5] ?? 0)) /
            3;
          return `${Math.round(x / 16)}:${Math.round(y / 16)}`;
        }
        return `${Math.round(shape.cx / 16)}:${Math.round(shape.cy / 16)}`;
      }),
    );

    expect(new Set(samples.map(({ shapes }) => shapes.length))).toEqual(
      new Set([4, 5, 6]),
    );
    expect(
      new Set(samples.map(({ shapes }) => shapes[0]?.shape.kind)).size,
    ).toBe(5);
    expect(signatures.size).toBeGreaterThanOrEqual(24);
    expect(anchorSignatures.size).toBeGreaterThanOrEqual(16);
  });

  test('keeps motion slow, visible, bounded, and meaningful', () => {
    const samples = Array.from({ length: 128 }, (_, index) =>
      deriveDocumentAtmosphere(`文档 ${index}`, 'light'),
    );

    for (const { motion, shape } of samples.flatMap(
      (atmosphere) => atmosphere.shapes,
    )) {
      expect(motion.duration).toBeGreaterThanOrEqual(56);
      expect(motion.duration).toBeLessThanOrEqual(130);
      expect(Math.abs(motion.driftX)).toBeLessThanOrEqual(31);
      expect(Math.abs(motion.driftY)).toBeLessThanOrEqual(21);
      expect(Math.abs(motion.swayX)).toBeLessThanOrEqual(31);
      expect(Math.abs(motion.swayY)).toBeLessThanOrEqual(21);
      if (motion.mode === 'float') {
        expect(Math.hypot(motion.driftX, motion.driftY)).toBeGreaterThanOrEqual(
          10,
        );
      }
      expect(motion.mode === 'spin').toBe(shape.kind === 'arc');
    }
  });

  test('only emits regular primitives inside the animated safe zone', () => {
    const samples = Array.from({ length: 128 }, (_, index) =>
      deriveDocumentAtmosphere(`版式 ${index}`, 'light'),
    );

    for (const { motion, shape } of samples.flatMap(
      (atmosphere) => atmosphere.shapes,
    )) {
      let left: number;
      let right: number;
      let top: number;
      let bottom: number;

      if (
        shape.kind === 'disk' ||
        shape.kind === 'ring' ||
        shape.kind === 'arc'
      ) {
        left = shape.cx - shape.r;
        right = shape.cx + shape.r;
        top = shape.cy - shape.r;
        bottom = shape.cy + shape.r;
        expect(shape.r).toBeGreaterThan(0);
        if (shape.kind === 'arc') {
          expect(shape.sweepDeg).toBeGreaterThanOrEqual(105);
          expect(shape.sweepDeg).toBeLessThanOrEqual(210);
        }
      } else if (shape.kind === 'roundedRect') {
        const centerX = shape.x + shape.width / 2;
        const centerY = shape.y + shape.height / 2;
        const extent = Math.hypot(shape.width, shape.height) / 2;
        left = centerX - extent;
        right = centerX + extent;
        top = centerY - extent;
        bottom = centerY + extent;
        expect(shape.width / shape.height).toBeGreaterThanOrEqual(1.44);
        expect(shape.width / shape.height).toBeLessThanOrEqual(2.01);
        expect(shape.rx).toBeLessThanOrEqual(shape.height * 0.22 + 0.01);
      } else {
        const coordinates = shape.points.match(/-?\d+(?:\.\d+)?/g)?.map(Number);
        expect(coordinates).toHaveLength(6);
        const points = [0, 2, 4].map((offset) => ({
          x: coordinates?.[offset] ?? 0,
          y: coordinates?.[offset + 1] ?? 0,
        }));
        const edges = points.map((point, index) => {
          const next = points[(index + 1) % points.length] ?? point;
          return Math.hypot(point.x - next.x, point.y - next.y);
        });
        left = Math.min(...points.map(({ x }) => x));
        right = Math.max(...points.map(({ x }) => x));
        top = Math.min(...points.map(({ y }) => y));
        bottom = Math.max(...points.map(({ y }) => y));
        expect(Math.max(...edges) - Math.min(...edges)).toBeLessThan(0.03);
      }

      const motionMarginX =
        Math.max(Math.abs(motion.driftX), Math.abs(motion.swayX)) * 0.65;
      const motionMarginY =
        Math.max(Math.abs(motion.driftY), Math.abs(motion.swayY)) * 0.65;
      expect(left - motionMarginX).toBeGreaterThanOrEqual(131.99);
      expect(right + motionMarginX).toBeLessThanOrEqual(356.01);
      expect(top - motionMarginY).toBeGreaterThanOrEqual(3.99);
      expect(bottom + motionMarginY).toBeLessThanOrEqual(96.01);
    }
  });
});
