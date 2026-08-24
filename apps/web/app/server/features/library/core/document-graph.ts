import { posix } from 'node:path';
import type { DocumentContent } from './types.js';
import { extractWikiLinkDocumentPaths } from './wiki-links.js';

export interface DocumentGraphNode {
  id: string;
  kind: 'attachment' | 'document';
  path: string;
  title: string;
  directory: string;
  inboundCount: number;
  outboundCount: number;
  degree: number;
}

export interface DocumentGraphEdge {
  id: string;
  kind: 'attachment' | 'hierarchy' | 'reference';
  source: string;
  target: string;
  count: number;
}

export interface DocumentGraph {
  nodes: DocumentGraphNode[];
  edges: DocumentGraphEdge[];
}

function parentDocumentPath(document: DocumentContent): string | null {
  if (!document.directory) return null;
  const parentDirectory = posix.dirname(document.directory);
  return parentDirectory === '.'
    ? 'index.md'
    : posix.join(parentDirectory, 'index.md');
}

export function buildDocumentGraph(
  documents: DocumentContent[],
  excludedPaths: ReadonlySet<string> = new Set(),
): DocumentGraph {
  const participatingDocuments = documents.filter(
    (document) => !excludedPaths.has(document.path),
  );
  const participatingPaths = new Set(
    participatingDocuments.map((document) => document.path),
  );

  const hierarchyEdges = participatingDocuments
    .flatMap((document) => {
      const parentPath = parentDocumentPath(document);
      if (!parentPath || !participatingPaths.has(parentPath)) return [];
      return [
        {
          id: '',
          kind: 'hierarchy' as const,
          source: parentPath,
          target: document.path,
          count: 1,
        },
      ];
    })
    .sort(
      (left, right) =>
        left.source.localeCompare(right.source) ||
        left.target.localeCompare(right.target),
    )
    .map((edge, index) => ({ ...edge, id: `hierarchy-edge-${index}` }));

  const referenceCounts = new Map<
    string,
    { source: string; target: string; count: number }
  >();
  for (const document of participatingDocuments) {
    for (const targetPath of extractWikiLinkDocumentPaths(document.content)) {
      if (targetPath === document.path || !participatingPaths.has(targetPath)) {
        continue;
      }
      const key = `${document.path}\0${targetPath}`;
      const existing = referenceCounts.get(key);
      if (existing) {
        existing.count += 1;
      } else {
        referenceCounts.set(key, {
          source: document.path,
          target: targetPath,
          count: 1,
        });
      }
    }
  }
  const referenceEdges = [...referenceCounts.values()]
    .sort(
      (left, right) =>
        left.source.localeCompare(right.source) ||
        left.target.localeCompare(right.target),
    )
    .map((edge, index) => ({
      ...edge,
      id: `reference-edge-${index}`,
      kind: 'reference' as const,
    }));

  const attachmentNodes: DocumentGraphNode[] = [];
  const attachmentEdges: DocumentGraphEdge[] = [];
  for (const document of participatingDocuments) {
    for (const asset of [...new Set(document.assets)].sort((left, right) =>
      left.localeCompare(right),
    )) {
      const id = `attachment:${encodeURIComponent(document.path)}:${encodeURIComponent(asset)}`;
      attachmentNodes.push({
        id,
        kind: 'attachment',
        path: posix.join(document.directory, asset),
        title: asset,
        directory: document.directory,
        inboundCount: 1,
        outboundCount: 0,
        degree: 1,
      });
      attachmentEdges.push({
        id: `attachment-edge-${attachmentEdges.length}`,
        kind: 'attachment',
        source: document.path,
        target: id,
        count: 1,
      });
    }
  }

  const edges = [...hierarchyEdges, ...referenceEdges, ...attachmentEdges];
  const inbound = new Map<string, number>();
  const outbound = new Map<string, number>();
  const neighbours = new Map<string, Set<string>>();
  for (const edge of edges) {
    inbound.set(edge.target, (inbound.get(edge.target) ?? 0) + 1);
    outbound.set(edge.source, (outbound.get(edge.source) ?? 0) + 1);
    if (!neighbours.has(edge.source)) neighbours.set(edge.source, new Set());
    if (!neighbours.has(edge.target)) neighbours.set(edge.target, new Set());
    neighbours.get(edge.source)?.add(edge.target);
    neighbours.get(edge.target)?.add(edge.source);
  }

  const nodes = participatingDocuments
    .map((document) => ({
      id: document.path,
      kind: 'document' as const,
      path: document.path,
      title: document.title,
      directory: document.directory,
      inboundCount: inbound.get(document.path) ?? 0,
      outboundCount: outbound.get(document.path) ?? 0,
      degree: neighbours.get(document.path)?.size ?? 0,
    }))
    .sort((left, right) => left.path.localeCompare(right.path));

  return {
    nodes: [...nodes, ...attachmentNodes],
    edges,
  };
}
