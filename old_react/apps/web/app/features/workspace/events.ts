import type { WorkspaceEventSnapshot } from '@locus/document-library/contracts/workspace-events';

export type { WorkspaceEventSnapshot };

export const WORKSPACE_EVENTS_PATH = '/api/workspace-events';

export function parseWorkspaceEventSnapshot(
  value: string,
): WorkspaceEventSnapshot | null {
  try {
    const snapshot = JSON.parse(value) as Partial<WorkspaceEventSnapshot>;
    if (
      (snapshot.graph !== undefined &&
        (!snapshot.graph ||
          !Array.isArray(snapshot.graph.nodes) ||
          !Array.isArray(snapshot.graph.edges))) ||
      !snapshot.status ||
      !snapshot.navigation ||
      !Array.isArray(snapshot.navigation.directories) ||
      !Array.isArray(snapshot.navigation.documents) ||
      !Array.isArray(snapshot.repositoryChanges)
    ) {
      return null;
    }
    return snapshot as WorkspaceEventSnapshot;
  } catch {
    return null;
  }
}
