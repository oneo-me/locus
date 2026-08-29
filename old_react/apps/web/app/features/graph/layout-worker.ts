export interface GraphLayoutNodeInput {
  id: string;
  x: number;
  y: number;
}

export interface GraphLayoutLinkInput {
  source: string;
  distance?: number;
  strength?: number;
  target: string;
}

export type GraphLayoutWorkerRequest =
  | {
      type: 'initialize';
      revision: number;
      nodes: GraphLayoutNodeInput[];
      links: GraphLayoutLinkInput[];
      reduceMotion: boolean;
    }
  | {
      type: 'update';
      revision: number;
      nodes: GraphLayoutNodeInput[];
      links: GraphLayoutLinkInput[];
    }
  | {
      type: 'pin';
      id: string;
      x: number;
      y: number;
    }
  | {
      type: 'release';
      id: string;
    };

export interface GraphLayoutWorkerResponse {
  type: 'positions';
  revision: number;
  positions: ArrayBuffer;
  settled: boolean;
}

export interface ResolvedGraphLayoutLink extends GraphLayoutLinkInput {
  strength: number;
}

export function graphLayoutOutputInterval(
  nodeCount: number,
  linkCount: number,
) {
  const complexity = Math.max(0, nodeCount) + Math.max(0, linkCount);
  if (complexity >= 4_000) return 1_000 / 20;
  if (complexity >= 1_200) return 1_000 / 30;
  return 1_000 / 60;
}

export function resolveGraphLayoutLinks(
  links: GraphLayoutLinkInput[],
): ResolvedGraphLayoutLink[] {
  const semanticDegrees = new Map<string, number>();
  for (const link of links) {
    if (link.strength !== undefined) continue;
    semanticDegrees.set(
      link.source,
      (semanticDegrees.get(link.source) ?? 0) + 1,
    );
    semanticDegrees.set(
      link.target,
      (semanticDegrees.get(link.target) ?? 0) + 1,
    );
  }

  return links.map((link) => ({
    ...link,
    strength:
      link.strength ??
      1 /
        Math.max(
          1,
          Math.min(
            semanticDegrees.get(link.source) ?? 1,
            semanticDegrees.get(link.target) ?? 1,
          ),
        ),
  }));
}

export function positionNewGraphNode(
  node: GraphLayoutNodeInput,
  links: GraphLayoutLinkInput[],
  positions: ReadonlyMap<string, { x: number; y: number }>,
) {
  const neighbours: Array<{ x: number; y: number }> = [];
  for (const link of links) {
    const neighbourId =
      link.source === node.id
        ? link.target
        : link.target === node.id
          ? link.source
          : null;
    if (!neighbourId) continue;
    const position = positions.get(neighbourId);
    if (position) neighbours.push(position);
  }
  if (neighbours.length === 0) return { x: node.x, y: node.y };

  const center = neighbours.reduce(
    (result, position) => ({
      x: result.x + position.x / neighbours.length,
      y: result.y + position.y / neighbours.length,
    }),
    { x: 0, y: 0 },
  );
  const angle = Math.atan2(node.y, node.x);
  return {
    x: center.x + Math.cos(angle) * 24,
    y: center.y + Math.sin(angle) * 24,
  };
}
