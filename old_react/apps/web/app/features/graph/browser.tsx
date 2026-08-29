import type { DocumentGraph } from '@locus/document-library/contracts/document-graph';
import type { LibraryGraph } from '@locus/document-library/contracts/library-api';
import {
  documentPathToUrlPath,
  urlPathToHref,
} from '@locus/document-library/contracts/library-route';
import { EllipsisIcon } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useLocation, useNavigate } from 'react-router';
import { useTheme } from '@locus/ui/theme-provider';
import { buttonVariants } from '@locus/ui/components/button';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from '@locus/ui/components/empty';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@locus/ui/components/popover';
import { cn } from '@locus/ui/utils';
import { useGlobalSettings } from '../settings/global-settings-provider';
import {
  parseWorkspaceEventSnapshot,
  type WorkspaceEventSnapshot,
  WORKSPACE_EVENTS_PATH,
} from '../workspace/events';
import { GraphAttachmentsControl } from './attachments-control';
import {
  type GraphCanvasEdge,
  type GraphCanvasNode,
  type GraphCanvasPalette,
  GraphCanvasRenderer,
} from './canvas-renderer';
import {
  createDirectoryGraphColorMap,
  createGraphPalette,
  type GraphTheme,
  topLevelDirectory,
} from './colors';
import { filterDocumentGraph } from './filter';
import {
  GRAPH_ATTACHMENT_LINK_DISTANCE,
  GRAPH_HIERARCHY_LINK_DISTANCE,
  GRAPH_REFERENCE_LINK_DISTANCE,
  graphNodeDiameter,
  initialGraphPosition,
} from './layout';
import type {
  GraphLayoutLinkInput,
  GraphLayoutNodeInput,
  GraphLayoutWorkerRequest,
  GraphLayoutWorkerResponse,
} from './layout-worker';

function useLiveGraphSnapshot(): WorkspaceEventSnapshot | null {
  const [snapshot, setSnapshot] = useState<WorkspaceEventSnapshot | null>(null);

  useEffect(() => {
    const events = new EventSource(`${WORKSPACE_EVENTS_PATH}?graph=1`);
    events.onmessage = (event) => {
      const nextSnapshot = parseWorkspaceEventSnapshot(event.data);
      if (nextSnapshot?.graph) setSnapshot(nextSnapshot);
    };
    return () => events.close();
  }, []);

  return snapshot;
}

export function GraphBrowser({ data }: { data: LibraryGraph }) {
  const snapshot = useLiveGraphSnapshot();
  const location = useLocation();
  const navigate = useNavigate();
  const liveData =
    snapshot?.graph === undefined
      ? data
      : { graph: snapshot.graph, status: snapshot.status };
  const { settings } = useGlobalSettings();
  const visibleGraph = useMemo(
    () =>
      filterDocumentGraph(liveData.graph, {
        showAttachments: settings.showGraphAttachments,
      }),
    [liveData.graph, settings.showGraphAttachments],
  );

  const documentCount = visibleGraph.nodes.filter(
    (node) => node.kind === 'document',
  ).length;
  const openDocument = useCallback(
    (documentPath: string) => {
      void navigate({
        pathname: urlPathToHref(documentPathToUrlPath(documentPath)),
        search: location.search,
      });
    },
    [location.search, navigate],
  );

  return (
    <main
      className="relative h-full min-h-0 min-w-0 overflow-hidden bg-card shadow-xs"
      id="main-content"
    >
      <p className="sr-only" aria-live="polite">
        {`知识图谱已更新，包含 ${visibleGraph.nodes.length} 个节点和 ${visibleGraph.edges.length} 条关系。`}
      </p>
      {documentCount === 0 ? (
        <GraphMessage
          title="还没有可绘制的文档"
          description={`在 ${liveData.status.rootPath} 中加入 index.md 后，知识图谱会自动更新。`}
        />
      ) : (
        <GraphCanvas graph={visibleGraph} onOpen={openDocument} />
      )}
      <GraphSettingsPopover />
    </main>
  );
}

function GraphSettingsPopover() {
  return (
    <div className="absolute top-2 right-2 z-10 md:top-4 md:right-4">
      <Popover>
        <PopoverTrigger
          aria-label="知识图谱设置"
          className={cn(
            buttonVariants({ size: 'icon-sm', variant: 'secondary' }),
          )}
        >
          <EllipsisIcon aria-hidden="true" />
        </PopoverTrigger>
        <PopoverContent align="end" className="w-72" side="bottom">
          <GraphAttachmentsControl />
        </PopoverContent>
      </Popover>
    </div>
  );
}

interface GraphPresentation {
  layoutNodes: GraphLayoutNodeInput[];
  links: GraphLayoutLinkInput[];
  renderNodes: GraphCanvasNode[];
  renderEdges: GraphCanvasEdge[];
}

interface GraphCanvasRuntime {
  appliedRevision: number;
  autoFitRevision: number | null;
  deferInitialGraph: boolean;
  graphSignature: string;
  initialized: boolean;
  pending: (GraphPresentation & { revision: number }) | null;
  ready: boolean;
  reduceMotion: boolean;
  renderer: GraphCanvasRenderer;
  revision: number;
  theme: GraphTheme;
  worker: Worker;
}

function currentDocumentTheme(): GraphTheme {
  return document.documentElement.classList.contains('dark') ? 'dark' : 'light';
}

function readGraphCanvasPalette(
  canvas: HTMLCanvasElement,
  theme: GraphTheme,
): GraphCanvasPalette {
  const palette = createGraphPalette(theme);

  return {
    background: getComputedStyle(canvas).backgroundColor,
    highlight: palette.highlight,
    highlightEdge: palette.highlightEdge,
    highlightLabel: palette.highlightLabel,
    mutedEdge: palette.document.fill,
    mutedNode: palette.document.fill,
  };
}

function GraphCanvas({
  graph,
  onOpen,
}: {
  graph: DocumentGraph;
  onOpen: (id: string) => void;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const runtime = useRef<GraphCanvasRuntime>(null);
  const graphRef = useRef(graph);
  const { resolvedTheme } = useTheme();
  const themeRef = useRef(resolvedTheme);

  useEffect(() => {
    graphRef.current = graph;
    themeRef.current = resolvedTheme;
  }, [graph, resolvedTheme]);

  useEffect(() => {
    if (!canvas.current) return;
    const host = canvas.current;
    const reduceMotion = window.matchMedia(
      '(prefers-reduced-motion: reduce)',
    ).matches;
    const layoutWorker = new Worker(
      new URL('./layout.worker.ts', import.meta.url),
      {
        name: 'Locus graph layout',
        type: 'module',
      },
    );
    const renderer = new GraphCanvasRenderer({
      canvas: host,
      nodes: [],
      edges: [],
      positions: new Float32Array(),
      palette: readGraphCanvasPalette(host, currentDocumentTheme()),
      reduceMotion,
      onPin: (id, x, y) => {
        const message: GraphLayoutWorkerRequest = { type: 'pin', id, x, y };
        layoutWorker.postMessage(message);
      },
      onRelease: (id) => {
        const message: GraphLayoutWorkerRequest = { type: 'release', id };
        layoutWorker.postMessage(message);
      },
      onOpen,
    });
    const nextRuntime: GraphCanvasRuntime = {
      appliedRevision: 0,
      autoFitRevision: null,
      deferInitialGraph: true,
      graphSignature: '',
      initialized: false,
      pending: null,
      ready: false,
      reduceMotion,
      renderer,
      revision: 0,
      theme: currentDocumentTheme(),
      worker: layoutWorker,
    };
    runtime.current = nextRuntime;

    layoutWorker.onmessage = (
      event: MessageEvent<GraphLayoutWorkerResponse>,
    ) => {
      if (event.data.type !== 'positions') return;
      const pending = nextRuntime.pending;
      if (!pending || event.data.revision !== pending.revision) return;
      const positions = new Float32Array(event.data.positions);
      if (nextRuntime.appliedRevision !== event.data.revision) {
        nextRuntime.appliedRevision = event.data.revision;
        renderer.updateGraph(
          pending.renderNodes,
          pending.renderEdges,
          positions,
        );
      } else {
        renderer.setPositions(positions, nextRuntime.autoFitRevision !== null);
      }
      if (!nextRuntime.ready) {
        nextRuntime.ready = true;
        renderer.fitGraph(true);
        renderer.setReady(true);
        nextRuntime.autoFitRevision = event.data.settled
          ? null
          : event.data.revision;
      } else if (nextRuntime.autoFitRevision !== null) {
        nextRuntime.autoFitRevision = event.data.revision;
        if (event.data.settled) {
          renderer.fitIfUntouched();
          nextRuntime.autoFitRevision = null;
        }
      }
      renderer.setLayoutSettled(event.data.settled);
    };
    const resizeObserver = new ResizeObserver(() => {
      renderer.resize();
    });
    resizeObserver.observe(host);

    // 先让浏览器绘制空画布承面，再开始图谱转换和 Worker 布局。这样切换视图
    // 不会和大图谱的同步准备工作挤在同一帧。
    const initialGraphFrame = requestAnimationFrame(() => {
      nextRuntime.deferInitialGraph = false;
      updateGraphCanvas(nextRuntime, graphRef.current, themeRef.current, host);
    });

    return () => {
      runtime.current = null;
      cancelAnimationFrame(initialGraphFrame);
      layoutWorker.terminate();
      resizeObserver.disconnect();
      renderer.destroy();
    };
  }, [onOpen]);

  useEffect(() => {
    if (
      canvas.current &&
      runtime.current &&
      !runtime.current.deferInitialGraph
    ) {
      updateGraphCanvas(runtime.current, graph, resolvedTheme, canvas.current);
    }
  }, [graph, resolvedTheme]);

  return (
    <canvas
      aria-label={`知识图谱，包含 ${graph.nodes.length} 个节点和 ${graph.edges.length} 条关系。双击文档节点可打开笔记，拖动画布可平移，滚动滚轮、按住 Control 或 Command 滚动及双指捏合可缩放。`}
      className="size-full touch-none bg-card motion-safe:animate-in motion-safe:fade-in motion-safe:duration-200"
      ref={canvas}
      role="img"
    />
  );
}

function updateGraphCanvas(
  runtime: GraphCanvasRuntime,
  graph: DocumentGraph,
  theme: GraphTheme,
  canvas: HTMLCanvasElement,
) {
  const signature = JSON.stringify(graph);
  if (signature === runtime.graphSignature) {
    // 图谱未变、仅主题切换：原地换色，保留布局、相机与悬停状态。
    if (theme === runtime.theme) return;
    runtime.theme = theme;
    const presentation = createGraphPresentation(graph, theme);
    if (runtime.pending) {
      runtime.pending = {
        ...runtime.pending,
        renderNodes: presentation.renderNodes,
        renderEdges: presentation.renderEdges,
      };
    }
    runtime.renderer.setPalette(readGraphCanvasPalette(canvas, theme));
    runtime.renderer.updateColors(
      presentation.renderNodes,
      presentation.renderEdges,
    );
    return;
  }
  runtime.graphSignature = signature;
  runtime.theme = theme;
  runtime.revision += 1;
  const presentation = createGraphPresentation(graph, theme);
  runtime.pending = { ...presentation, revision: runtime.revision };
  const message: GraphLayoutWorkerRequest = runtime.initialized
    ? {
        type: 'update',
        revision: runtime.revision,
        nodes: presentation.layoutNodes,
        links: presentation.links,
      }
    : {
        type: 'initialize',
        revision: runtime.revision,
        nodes: presentation.layoutNodes,
        links: presentation.links,
        reduceMotion: runtime.reduceMotion,
      };
  runtime.initialized = true;
  runtime.worker.postMessage(message);
}

function createGraphPresentation(
  graph: DocumentGraph,
  theme: GraphTheme,
): GraphPresentation {
  const palette = createGraphPalette(theme);
  const directoryColorMap = createDirectoryGraphColorMap(
    graph.nodes
      .filter((node) => node.kind === 'document')
      .map((node) => topLevelDirectory(node.path)),
    theme,
  );
  const directoryClusters = [
    ...new Set(
      graph.nodes.map((node) => topLevelDirectory(node.path)).filter(Boolean),
    ),
  ].sort((left, right) => left.localeCompare(right));
  const directoryClusterIndex = new Map(
    directoryClusters.map((directory, index) => [directory, index]),
  );
  const layoutNodes = graph.nodes.map((node) => {
    const directory = topLevelDirectory(node.path);
    const clusterIndex = directory
      ? directoryClusterIndex.get(directory)
      : undefined;
    return {
      id: node.id,
      ...initialGraphPosition(
        node,
        graph.nodes.length,
        clusterIndex === undefined
          ? undefined
          : {
              index: clusterIndex,
              count: directoryClusters.length,
            },
      ),
    };
  });
  const renderNodes: GraphCanvasNode[] = graph.nodes.map((node) => {
    if (node.kind === 'attachment') {
      return {
        id: node.id,
        label: node.title,
        fill: palette.attachment.fill,
        labelColor: palette.attachment.label,
        diameter: 8,
      };
    }
    const directory = topLevelDirectory(node.path);
    const directoryColor = directory
      ? directoryColorMap.get(directory)
      : undefined;
    if (directoryColor) {
      return {
        id: node.id,
        label: node.title,
        fill: directoryColor.fill,
        labelColor: directoryColor.label,
        diameter: graphNodeDiameter(node.degree),
        openable: true,
      };
    }
    return {
      id: node.id,
      label: node.title,
      fill:
        node.degree === 0
          ? palette.documentIsolated.fill
          : palette.document.fill,
      labelColor:
        node.degree === 0
          ? palette.documentIsolated.label
          : palette.document.label,
      diameter: graphNodeDiameter(node.degree),
      openable: true,
    };
  });
  const nodeIndexes = new Map(
    graph.nodes.map((node, index) => [node.id, index]),
  );
  const renderEdges: GraphCanvasEdge[] = graph.edges.flatMap((edge) => {
    const source = nodeIndexes.get(edge.source);
    const target = nodeIndexes.get(edge.target);
    if (source === undefined || target === undefined) return [];
    if (edge.kind === 'attachment') {
      return [
        {
          source,
          target,
          color: palette.attachment.fill,
          opacity: palette.attachmentEdgeOpacity,
          lineWidth: 0.8,
        },
      ];
    }
    const sourceNode = graph.nodes[source];
    const sourceDirectory = sourceNode
      ? topLevelDirectory(sourceNode.path)
      : '';
    const sourceColor = sourceDirectory
      ? directoryColorMap.get(sourceDirectory)
      : undefined;
    if (edge.kind === 'reference') {
      return [
        {
          source,
          target,
          color: sourceColor?.fill ?? palette.document.fill,
          opacity: palette.referenceEdgeOpacity,
          lineWidth: 0.85,
          hoverOpacity: 0.35,
        },
      ];
    }
    return [
      {
        source,
        target,
        color: sourceColor?.fill ?? palette.document.fill,
        opacity: palette.hierarchyEdgeOpacity,
        lineWidth: 1,
        hoverOpacity: 0.4,
      },
    ];
  });

  return {
    layoutNodes,
    links: graph.edges.map((edge) => ({
      source: edge.source,
      distance:
        edge.kind === 'attachment'
          ? GRAPH_ATTACHMENT_LINK_DISTANCE
          : edge.kind === 'reference'
            ? GRAPH_REFERENCE_LINK_DISTANCE
            : GRAPH_HIERARCHY_LINK_DISTANCE,
      target: edge.target,
    })),
    renderNodes,
    renderEdges,
  };
}

function GraphMessage({
  title,
  description,
}: {
  title: string;
  description: string;
}) {
  return (
    <Empty
      className="mx-auto mt-[18vh] w-[calc(100%-3rem)] max-w-xl flex-none"
      role="status"
    >
      <EmptyHeader>
        <EmptyTitle>{title}</EmptyTitle>
        <EmptyDescription>{description}</EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}
