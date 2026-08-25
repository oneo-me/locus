import {
  documentPathToUrlPath,
  urlPathToHref,
} from '@locus/document-library/contracts/library-route';
import { useGlobalSettings } from '~/features/settings/global-settings-provider';
import { WorkspaceDocumentLandingState } from '~/features/workspace/route-state';
import { useWorkspaceLibraryData } from '~/features/workspace/shell';
import { Navigate } from 'react-router';

export default function WorkspaceIndexRoute() {
  const data = useWorkspaceLibraryData();
  const { settings } = useGlobalSettings();
  const defaultDocumentPath = settings.defaultDocumentPath;

  if (
    defaultDocumentPath &&
    data.documents.some((document) => document.path === defaultDocumentPath)
  ) {
    return (
      <Navigate
        replace
        to={urlPathToHref(documentPathToUrlPath(defaultDocumentPath))}
      />
    );
  }

  return <WorkspaceDocumentLandingState data={data} />;
}
