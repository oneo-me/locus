import type { DocumentGraphNode } from '~/contracts/document-graph';

/** Obsidian-aligned global forces — keeps the familiar overall spacing and air. */
export const GRAPH_FORCE_DEFAULTS = {
  centerStrength: 0.1,
  collisionRadius: 60,
  collisionStrength: 0.5,
  linkDistance: 250,
  repelStrength: -1_000,
} as const;

/** Parent/child edges stay tighter than the global default spacing. */
export const GRAPH_HIERARCHY_LINK_DISTANCE = 160;

/** Wiki reference edges sit between hierarchy clusters and the global spacing. */
export const GRAPH_REFERENCE_LINK_DISTANCE = 200;

/** Attachments sit close to their host document. */
export const GRAPH_ATTACHMENT_LINK_DISTANCE = 64;

const INITIAL_AREA_PER_NODE = 60 * 60;
const MIN_NODE_RADIUS = 6;
const MAX_NODE_RADIUS = 22;
const MIN_SCREEN_NODE_RADIUS = 0.75;
const NODE_SHRINK_ZOOM_THRESHOLD = 0.4;

export interface GraphLayoutCluster {
  index: number;
  count: number;
}

function hashValue(value: string) {
  let hash = 2166136261;
  for (const character of value) {
    hash ^= character.charCodeAt(0);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

/**
 * Uses a compact version of Obsidian's node sizing curve.
 */
export function graphNodeDiameter(weight: number) {
  const radius = Math.max(
    MIN_NODE_RADIUS,
    Math.min(2.5 * Math.sqrt(Math.max(0, weight) + 1), MAX_NODE_RADIUS),
  );
  return radius * 2;
}

/**
 * Keeps nodes stable at normal zoom levels, then scales them down with the
 * graph once the camera enters a distant overview.
 */
export function graphNodeScreenRadius(diameter: number, zoom: number) {
  const scale = Math.min(1, zoom / NODE_SHRINK_ZOOM_THRESHOLD);
  return Math.max(MIN_SCREEN_NODE_RADIUS, (diameter / 2) * scale);
}

/**
 * Seeds nodes in Obsidian's area envelope. When a top-level directory cluster
 * is provided, same-directory nodes start near each other instead of scattered
 * across the whole disk.
 */
export function initialGraphPosition(
  node: Pick<DocumentGraphNode, 'id'>,
  nodeCount: number,
  cluster?: GraphLayoutCluster,
) {
  const hash = hashValue(node.id);
  const localAngle = ((hash & 0xffff) / 0x10000) * Math.PI * 2;
  const radialSample = (((hash >>> 16) & 0xffff) + 0.5) / 0x10000;
  const outerRadius = Math.sqrt(
    (Math.max(1, nodeCount) * INITIAL_AREA_PER_NODE) / Math.PI,
  );

  if (!cluster || cluster.count <= 1) {
    const radius = Math.sqrt(radialSample) * outerRadius;
    return {
      x: Math.cos(localAngle) * radius,
      y: Math.sin(localAngle) * radius,
    };
  }

  const clusterAngle = (cluster.index / cluster.count) * Math.PI * 2;
  const clusterRing = outerRadius * 0.72;
  const nodesPerCluster = Math.max(1, nodeCount / cluster.count);
  const clusterRadius = Math.max(
    64,
    Math.sqrt((nodesPerCluster * INITIAL_AREA_PER_NODE) / Math.PI) * 0.95,
  );
  const localRadius = Math.sqrt(radialSample) * clusterRadius;

  return {
    x:
      Math.cos(clusterAngle) * clusterRing + Math.cos(localAngle) * localRadius,
    y:
      Math.sin(clusterAngle) * clusterRing + Math.sin(localAngle) * localRadius,
  };
}
