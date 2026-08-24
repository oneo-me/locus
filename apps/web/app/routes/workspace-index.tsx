import { WorkspaceDocumentLandingState } from '~/features/workspace/route-state';
import { useWorkspaceLibraryData } from '~/features/workspace/shell';

export default function WorkspaceIndexRoute() {
  return <WorkspaceDocumentLandingState data={useWorkspaceLibraryData()} />;
}
