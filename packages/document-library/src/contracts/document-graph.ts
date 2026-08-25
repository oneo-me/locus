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
