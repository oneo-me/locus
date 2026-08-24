import { describe, expect, test } from 'vitest';
import {
  createDirectoryGraphColorMap,
  createGraphPalette,
  topLevelDirectory,
} from './colors';

function hueOf(fill: string) {
  const match = /^hsl\(([\d.]+),\s*[\d.]+%,\s*[\d.]+%\)$/.exec(fill);
  if (!match?.[1]) throw new Error(`unexpected fill: ${fill}`);
  return Number(match[1]);
}

describe('topLevelDirectory', () => {
  test('returns the first path segment for nested documents', () => {
    expect(topLevelDirectory('projects/asterweft/features/index.md')).toBe(
      'projects',
    );
    expect(topLevelDirectory('journal/2026/index.md')).toBe('journal');
  });

  test('treats the library root document as having no top-level directory', () => {
    expect(topLevelDirectory('index.md')).toBe('');
  });
});

describe('createDirectoryGraphColorMap', () => {
  test('ignores input order and assigns distinct colors for distinct directories', () => {
    const colors = createDirectoryGraphColorMap([
      'projects',
      'journal',
      'projects',
      '',
    ]);
    const reorderedColors = createDirectoryGraphColorMap([
      'journal',
      'projects',
    ]);

    expect(colors).toHaveLength(2);
    expect(colors.get('projects')).toEqual(reorderedColors.get('projects'));
    expect(colors.get('journal')).toEqual(reorderedColors.get('journal'));
    expect(colors.get('projects')?.fill).not.toBe(colors.get('journal')?.fill);
  });

  test('spaces directory hues evenly so no two colors are too close', () => {
    const directories = ['projects', 'journal', 'notes', 'ideas', 'meetings'];
    const colors = createDirectoryGraphColorMap(directories);
    const hues = [...colors.values()]
      .map((color) => hueOf(color.fill))
      .sort((left, right) => left - right);
    const expectedGap = 360 / directories.length;

    for (let index = 0; index < hues.length; index += 1) {
      const current = hues[index] ?? 0;
      const next =
        (hues[(index + 1) % hues.length] ?? 0) +
        (index === hues.length - 1 ? 360 : 0);
      expect(next - current).toBeCloseTo(expectedGap);
    }
  });

  test('is deterministic for the same set regardless of input order', () => {
    const directories = ['journal', 'projects', 'notes'];
    expect(createDirectoryGraphColorMap(directories)).toEqual(
      createDirectoryGraphColorMap([...directories].reverse()),
    );
  });
});

describe('createGraphPalette', () => {
  test('keeps attachment edges quieter than reference and hierarchy edges', () => {
    for (const theme of ['light', 'dark'] as const) {
      const palette = createGraphPalette(theme);
      expect(palette.attachmentEdgeOpacity).toBeLessThan(
        palette.referenceEdgeOpacity,
      );
      expect(palette.referenceEdgeOpacity).toBeLessThan(
        palette.hierarchyEdgeOpacity,
      );
    }
  });
});
