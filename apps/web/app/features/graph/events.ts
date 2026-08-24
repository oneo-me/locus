import type { GraphEventSnapshot } from '@locus/document-library/contracts/graph-events';

export type { GraphEventSnapshot };

export const GRAPH_EVENTS_PATH = '/api/graph-events';

export function parseGraphEventSnapshot(
  value: string,
): GraphEventSnapshot | null {
  try {
    const snapshot = JSON.parse(value) as Partial<GraphEventSnapshot>;
    if (
      (snapshot.graph !== undefined &&
        (!snapshot.graph ||
          !Array.isArray(snapshot.graph.nodes) ||
          !Array.isArray(snapshot.graph.edges))) ||
      !snapshot.navigation ||
      !Array.isArray(snapshot.navigation.directories) ||
      !Array.isArray(snapshot.navigation.documents) ||
      !Array.isArray(snapshot.repositoryChanges) ||
      (snapshot.repositoryError !== null &&
        typeof snapshot.repositoryError !== 'string') ||
      (snapshot.repositoryRevision !== null &&
        typeof snapshot.repositoryRevision !== 'string') ||
      !snapshot.status ||
      typeof snapshot.status.documentCount !== 'number' ||
      typeof snapshot.status.refreshing !== 'boolean'
    ) {
      return null;
    }
    return snapshot as GraphEventSnapshot;
  } catch {
    return null;
  }
}
