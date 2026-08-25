export interface GraphColorSet {
  fill: string;
  label: string;
}

export type GraphTheme = 'light' | 'dark';

/** 节点、连线与高亮的整套配色，按主题成组提供。 */
export interface GraphPalette {
  attachment: GraphColorSet;
  attachmentEdgeOpacity: number;
  document: GraphColorSet;
  documentIsolated: Pick<GraphColorSet, 'fill' | 'label'>;
  hierarchyEdgeOpacity: number;
  highlight: string;
  highlightEdge: string;
  highlightLabel: string;
  referenceEdgeOpacity: number;
}

function hashValue(value: string) {
  let hash = 2166136261;
  for (const character of value) {
    hash ^= character.charCodeAt(0);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

function directoryColors(
  directory: string,
  hue: number,
  theme: GraphTheme,
): GraphColorSet {
  const hash = hashValue(directory);
  const roundedHue = Math.round(hue * 10) / 10;

  if (theme === 'dark') {
    const saturation = 50 + ((hash >>> 8) % 28);
    const lightness = 56 + ((hash >>> 16) % 18);

    return {
      fill: `hsl(${roundedHue}, ${saturation}%, ${lightness}%)`,
      label: `hsl(${roundedHue}, 28%, 82%)`,
    };
  }

  const saturation = 48 + ((hash >>> 8) % 28);
  const lightness = 36 + ((hash >>> 16) % 18);

  return {
    fill: `hsl(${roundedHue}, ${saturation}%, ${lightness}%)`,
    label: `hsl(${roundedHue}, 38%, 28%)`,
  };
}

/** 文档路径的顶层目录：`projects/foo/index.md` → `projects`；根文档为空。 */
export function topLevelDirectory(path: string): string {
  const normalized = path.replaceAll('\\', '/').replace(/^\.\/+/, '');
  if (!normalized || normalized === 'index.md') return '';
  const firstSegment = normalized.split('/')[0] ?? '';
  return firstSegment === 'index.md' ? '' : firstSegment;
}

/** 按排序后的目录名均分色环，保证同一库内任意两个目录的色相都有足够间距。 */
export function createDirectoryGraphColorMap(
  directories: string[],
  theme: GraphTheme = 'light',
) {
  const uniqueDirectories = [
    ...new Set(directories.map((value) => value.trim())),
  ]
    .filter(Boolean)
    .sort();

  return new Map(
    uniqueDirectories.map((directory, index) => {
      const hue = (208 + (index * 360) / uniqueDirectories.length) % 360;
      return [directory, directoryColors(directory, hue, theme)];
    }),
  );
}

const LIGHT_GRAPH_PALETTE: GraphPalette = {
  attachment: { fill: '#e0b35a', label: '#8a6a2a' },
  attachmentEdgeOpacity: 0.13,
  document: { fill: '#626a70', label: '#3f474d' },
  documentIsolated: { fill: '#7f878d', label: '#596168' },
  hierarchyEdgeOpacity: 0.2,
  highlight: '#028bff',
  highlightEdge: '#6f8fa3',
  highlightLabel: '#166aa8',
  referenceEdgeOpacity: 0.16,
};

const DARK_GRAPH_PALETTE: GraphPalette = {
  attachment: { fill: '#d4b06a', label: '#ead7a8' },
  attachmentEdgeOpacity: 0.17,
  document: { fill: '#9aa7b3', label: '#d4dbe2' },
  documentIsolated: { fill: '#7d8894', label: '#b8c0c8' },
  hierarchyEdgeOpacity: 0.25,
  highlight: '#5aadef',
  highlightEdge: '#8eb4cc',
  highlightLabel: '#b5d8f0',
  referenceEdgeOpacity: 0.2,
};

export function createGraphPalette(theme: GraphTheme): GraphPalette {
  return theme === 'dark' ? DARK_GRAPH_PALETTE : LIGHT_GRAPH_PALETTE;
}
