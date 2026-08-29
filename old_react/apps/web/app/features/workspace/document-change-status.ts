import type { LibraryNavigationDocument } from '@locus/document-library/contracts/library-api';
import type { LibraryRepositoryChange } from '@locus/document-library/contracts/repository';
import type { DocumentTreeNode } from '../library/tree';

export type DocumentChangeStatus = 'added' | 'modified';

export interface DocumentTreeChangeState {
  own: DocumentChangeStatus | null;
  descendants: DocumentChangeStatus[];
}

const documentChangeStatuses: DocumentChangeStatus[] = ['added', 'modified'];

function parentDirectory(path: string): string {
  const separator = path.lastIndexOf('/');
  return separator < 0 ? '' : path.slice(0, separator);
}

export function getDocumentChangeStatuses(
  documents: LibraryNavigationDocument[],
  changes: LibraryRepositoryChange[],
): Map<string, DocumentChangeStatus> {
  const statuses = new Map<string, DocumentChangeStatus>();
  for (const document of documents) {
    const directory = parentDirectory(document.path);
    for (const change of changes) {
      const changesDocument =
        change.file === document.path || change.previousFile === document.path;
      const changesAttachment =
        parentDirectory(change.file) === directory ||
        (change.previousFile !== undefined &&
          parentDirectory(change.previousFile) === directory);
      if (!changesDocument && !changesAttachment) continue;
      const status =
        changesDocument && change.status === 'added' ? 'added' : 'modified';
      if (status === 'added' || !statuses.has(document.path)) {
        statuses.set(document.path, status);
      }
    }
  }
  return statuses;
}

export function getDocumentTreeChangeStates(
  nodes: DocumentTreeNode[],
  documentStatuses: ReadonlyMap<string, DocumentChangeStatus>,
): Map<string, DocumentTreeChangeState> {
  const states = new Map<string, DocumentTreeChangeState>();

  const collect = (node: DocumentTreeNode): Set<DocumentChangeStatus> => {
    const own = node.document
      ? (documentStatuses.get(node.document.path) ?? null)
      : null;
    const descendantStatuses = new Set<DocumentChangeStatus>();

    for (const child of node.children) {
      for (const status of collect(child)) descendantStatuses.add(status);
    }

    if (own || descendantStatuses.size > 0) {
      states.set(node.path, {
        own,
        descendants: documentChangeStatuses.filter((status) =>
          descendantStatuses.has(status),
        ),
      });
    }

    if (own) descendantStatuses.add(own);
    return descendantStatuses;
  };

  for (const node of nodes) collect(node);
  return states;
}

export function getDocumentTreeChangeLabel(
  state: DocumentTreeChangeState,
): string {
  if (state.own === 'added') return '新增，尚未提交';
  if (state.own === 'modified') return '已修改，尚未提交';

  const descendants = new Set(state.descendants);
  if (descendants.has('added') && descendants.has('modified')) {
    return '子文章有新增和修改，尚未提交';
  }
  if (descendants.has('added')) return '子文章有新增，尚未提交';
  if (descendants.has('modified')) return '子文章有修改，尚未提交';

  return '';
}
