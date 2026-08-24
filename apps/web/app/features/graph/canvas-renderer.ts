import { type Selection, select } from 'd3-selection';
import {
  type D3ZoomEvent,
  type ZoomBehavior,
  zoom,
  zoomIdentity,
} from 'd3-zoom';
import { graphNodeScreenRadius } from './layout';

export interface GraphCanvasNode {
  id: string;
  label: string;
  fill: string;
  labelColor: string;
  diameter: number;
  openable?: boolean;
}

export interface GraphCanvasEdge {
  dash?: number[];
  hoverOpacity?: number;
  lineWidth?: number;
  source: number;
  target: number;
  color: string;
  opacity: number;
}

/** 画布背景与高亮色，随主题整体替换。 */
export interface GraphCanvasPalette {
  background: string;
  highlight: string;
  highlightEdge: string;
  highlightLabel: string;
  mutedEdge: string;
  mutedNode: string;
}

export interface GraphCanvasRendererOptions {
  canvas: HTMLCanvasElement;
  nodes: GraphCanvasNode[];
  edges: GraphCanvasEdge[];
  positions: Float32Array;
  palette: GraphCanvasPalette;
  reduceMotion: boolean;
  onPin(id: string, x: number, y: number): void;
  onRelease(id: string): void;
  onOpen(id: string): void;
}

const MIN_ZOOM = 1 / 128;
const MAX_ZOOM = 8;
const LABEL_FADE_START = 0.35;
const LABEL_FADE_END = 0.65;
const FIT_PADDING = 72;
const CAMERA_RESPONSE_MS = 72;
const HOVER_TRANSITION_MS = 180;
export const GRAPH_REVEAL_DURATION_MS = 260;
const POSITION_EPSILON = 0.08;
const ZOOM_EPSILON = 0.000_05;
const HOVER_EPSILON = 0.001;
const MUTED_NODE_OPACITY = 0.08;
const TRACKPAD_ZOOM_DISTANCE = 48;
const MOUSE_WHEEL_ZOOM_DISTANCE = 120;
/** 悬停时未关联连线的透明度下限。 */
const MUTED_EDGE_OPACITY = 0.04;
/** 悬停时完全关联连线的透明度上限，用于相对默认态真正“亮起来”。 */
const HOVER_EDGE_PEAK_OPACITY = 0.92;
const MUTED_LABEL_OPACITY = 0.1;
const LABEL_MAX_WIDTH = 132;
/** 悬停时仅高亮当前节点及直接相邻节点。 */
export const HOVER_DEPTH = 1;
export const HOVER_NODE_EMPHASIS = [1, 0.8] as const;
const CLICK_MOVE_TOLERANCE = 4;
const WHEEL_DELTA_LINE = 1;
const WHEEL_DELTA_PAGE = 2;

type GraphWheelEvent = Pick<WheelEvent, 'ctrlKey' | 'deltaMode' | 'deltaY'>;

export interface GraphWheelInteraction {
  delta: number;
  source: 'pinch' | 'wheel';
}

function normalizedWheelDelta(event: GraphWheelEvent, pageHeight: number) {
  const scale =
    event.deltaMode === WHEEL_DELTA_LINE
      ? 16
      : event.deltaMode === WHEEL_DELTA_PAGE
        ? pageHeight
        : 1;
  return event.deltaY * scale;
}

/** 滚轮、触摸板滚动与捏合统一映射为以指针为中心的缩放。 */
export function graphWheelInteraction(
  event: GraphWheelEvent,
  pageHeight: number,
): GraphWheelInteraction {
  return {
    delta: normalizedWheelDelta(event, pageHeight),
    source: event.ctrlKey && event.deltaMode === 0 ? 'pinch' : 'wheel',
  };
}

export function graphRevealNodeCount(
  nodeCount: number,
  elapsed: number,
  duration = GRAPH_REVEAL_DURATION_MS,
) {
  if (nodeCount <= 0) return 0;
  if (duration <= 0 || elapsed >= duration) return nodeCount;
  if (elapsed < 0) return 0;
  const progress = clamp(elapsed / duration, 0, 1);
  const easedProgress = 1 - (1 - progress) ** 2;
  return Math.max(1, Math.min(nodeCount, Math.ceil(nodeCount * easedProgress)));
}

interface Camera {
  x: number;
  y: number;
  zoom: number;
}

interface GraphCenter {
  x: number;
  y: number;
}

type PointerInteraction = {
  dragging: boolean;
  index: number;
  offsetX: number;
  offsetY: number;
};

type ClickCandidate = {
  moved: boolean;
  pointerId: number;
  startX: number;
  startY: number;
};

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(maximum, Math.max(minimum, value));
}

export function graphBoundsCenter(positions: Float32Array): GraphCenter | null {
  if (positions.length < 2) return null;

  let minimumX = Number.POSITIVE_INFINITY;
  let maximumX = Number.NEGATIVE_INFINITY;
  let minimumY = Number.POSITIVE_INFINITY;
  let maximumY = Number.NEGATIVE_INFINITY;
  for (let index = 0; index + 1 < positions.length; index += 2) {
    const x = positions[index];
    const y = positions[index + 1];
    if (x === undefined || y === undefined) continue;
    minimumX = Math.min(minimumX, x);
    maximumX = Math.max(maximumX, x);
    minimumY = Math.min(minimumY, y);
    maximumY = Math.max(maximumY, y);
  }

  if (!Number.isFinite(minimumX) || !Number.isFinite(minimumY)) return null;
  return {
    x: (minimumX + maximumX) / 2,
    y: (minimumY + maximumY) / 2,
  };
}

export function compensateCameraForGraphCenter(
  camera: Camera,
  previousCenter: GraphCenter,
  nextCenter: GraphCenter,
): Camera {
  return {
    ...camera,
    x: camera.x + (previousCenter.x - nextCenter.x) * camera.zoom,
    y: camera.y + (previousCenter.y - nextCenter.y) * camera.zoom,
  };
}

function parseGraphColor(color: string): [number, number, number] | null {
  const hexMatch = /^#([\da-f]{6})$/i.exec(color);
  if (hexMatch?.[1]) {
    const value = Number.parseInt(hexMatch[1], 16);
    return [(value >> 16) & 0xff, (value >> 8) & 0xff, value & 0xff];
  }

  const rgbMatch =
    /^rgba?\(\s*(-?[\d.]+)\s*[,\s]\s*(-?[\d.]+)\s*[,\s]\s*(-?[\d.]+)(?:\s*[/,]\s*(-?[\d.]+%?))?\s*\)$/i.exec(
      color,
    );
  if (rgbMatch?.[1] && rgbMatch[2] && rgbMatch[3]) {
    return [
      clamp(Math.round(Number(rgbMatch[1])), 0, 255),
      clamp(Math.round(Number(rgbMatch[2])), 0, 255),
      clamp(Math.round(Number(rgbMatch[3])), 0, 255),
    ];
  }

  const hslMatch =
    /^hsl\(\s*(-?[\d.]+)\s*,\s*([\d.]+)%\s*,\s*([\d.]+)%\s*\)$/i.exec(color);
  if (!hslMatch?.[1] || !hslMatch[2] || !hslMatch[3]) return null;

  const hue = (((Number(hslMatch[1]) % 360) + 360) % 360) / 60;
  const saturation = clamp(Number(hslMatch[2]) / 100, 0, 1);
  const lightness = clamp(Number(hslMatch[3]) / 100, 0, 1);
  const chroma = (1 - Math.abs(2 * lightness - 1)) * saturation;
  const secondary = chroma * (1 - Math.abs((hue % 2) - 1));
  const [red, green, blue] =
    hue < 1
      ? [chroma, secondary, 0]
      : hue < 2
        ? [secondary, chroma, 0]
        : hue < 3
          ? [0, chroma, secondary]
          : hue < 4
            ? [0, secondary, chroma]
            : hue < 5
              ? [secondary, 0, chroma]
              : [chroma, 0, secondary];
  const offset = lightness - chroma / 2;
  return [red, green, blue].map((channel) =>
    Math.round((channel + offset) * 255),
  ) as [number, number, number];
}

export function mixGraphColor(
  start: string,
  end: string,
  amount: number,
): string {
  const progress = clamp(amount, 0, 1);
  const startChannels = parseGraphColor(start);
  const endChannels = parseGraphColor(end);
  if (!startChannels || !endChannels) return progress < 1 ? start : end;
  const channel = (index: number) =>
    Math.round(
      (startChannels[index] ?? 0) * (1 - progress) +
        (endChannels[index] ?? 0) * progress,
    );
  return `rgb(${channel(0)}, ${channel(1)}, ${channel(2)})`;
}

/** Soft radial fill: light highlight toward the top-left, deeper rim. */
export function createGraphNodeFill(
  context: CanvasRenderingContext2D,
  x: number,
  y: number,
  radius: number,
  fill: string,
) {
  const gradient = context.createRadialGradient(
    x - radius * 0.32,
    y - radius * 0.36,
    Math.max(0.5, radius * 0.06),
    x,
    y,
    Math.max(1, radius),
  );
  gradient.addColorStop(0, mixGraphColor(fill, '#ffffff', 0.34));
  gradient.addColorStop(0.52, fill);
  gradient.addColorStop(1, mixGraphColor(fill, '#000000', 0.2));
  return gradient;
}

export function fitGraphLabel(
  label: string,
  maximumWidth: number,
  measure: (value: string) => number,
): string {
  if (measure(label) <= maximumWidth) return label;
  const characters = Array.from(label);
  const ellipsis = '…';
  if (measure(ellipsis) > maximumWidth) return '';

  let minimum = 0;
  let maximum = characters.length;
  while (minimum < maximum) {
    const middle = Math.ceil((minimum + maximum) / 2);
    const candidate = `${characters.slice(0, middle).join('')}${ellipsis}`;
    if (measure(candidate) <= maximumWidth) minimum = middle;
    else maximum = middle - 1;
  }
  return `${characters.slice(0, minimum).join('')}${ellipsis}`;
}

export function buildGraphAdjacency(
  nodeCount: number,
  edges: Array<{ source: number; target: number }>,
): number[][] {
  const adjacency = Array.from({ length: nodeCount }, () => [] as number[]);
  for (const edge of edges) {
    adjacency[edge.source]?.push(edge.target);
    adjacency[edge.target]?.push(edge.source);
  }
  return adjacency;
}

/** 以 root 为中心做 BFS，返回节点到 root 的跳数（含 0），最远不超过 maxDepth。 */
export function computeHoverDepths(
  adjacency: number[][],
  root: number,
  maxDepth = HOVER_DEPTH,
): Map<number, number> {
  const depths = new Map<number, number>();
  if (root < 0 || root >= adjacency.length) return depths;

  const queue = [root];
  depths.set(root, 0);
  for (let head = 0; head < queue.length; head += 1) {
    const node = queue[head];
    if (node === undefined) continue;
    const depth = depths.get(node) ?? 0;
    if (depth >= maxDepth) continue;
    for (const neighbor of adjacency[node] ?? []) {
      if (depths.has(neighbor)) continue;
      depths.set(neighbor, depth + 1);
      queue.push(neighbor);
    }
  }
  return depths;
}

export function hoverEmphasisForDepth(depth: number | undefined): number {
  if (depth === undefined) return 0;
  return HOVER_NODE_EMPHASIS[depth] ?? 0;
}

/** 在悬停邻域切换时，平滑计算节点从原色过渡到中性灰的程度。 */
export function hoverMuteForNode(
  previousDepths: Map<number, number> | null | undefined,
  currentDepths: Map<number, number> | null | undefined,
  node: number,
  transition: number,
): number {
  const muteFor = (depths: Map<number, number> | null | undefined) =>
    depths && !depths.has(node) ? 1 : 0;
  const previous = muteFor(previousDepths);
  const current = muteFor(currentDepths);
  const progress = clamp(transition, 0, 1);
  return previous + (current - previous) * progress;
}

/** 在悬停邻域切换时，平滑计算连线从原色过渡到中性灰的程度。 */
export function hoverMuteForEdge(
  previousDepths: Map<number, number> | null | undefined,
  currentDepths: Map<number, number> | null | undefined,
  source: number,
  target: number,
  transition: number,
): number {
  const muteFor = (depths: Map<number, number> | null | undefined) =>
    depths && (!depths.has(source) || !depths.has(target)) ? 1 : 0;
  const previous = muteFor(previousDepths);
  const current = muteFor(currentDepths);
  const progress = clamp(transition, 0, 1);
  return previous + (current - previous) * progress;
}

/** 两端都在邻域内时，按较远一端的层级取强调；否则视为未关联。 */
export function hoverEmphasisForEdge(
  depths: Map<number, number> | null | undefined,
  source: number,
  target: number,
): number {
  if (!depths) return 0;
  const sourceDepth = depths.get(source);
  const targetDepth = depths.get(target);
  if (sourceDepth === undefined || targetDepth === undefined) return 0;
  return hoverEmphasisForDepth(Math.max(sourceDepth, targetDepth));
}

export class GraphCanvasRenderer {
  private readonly canvas: HTMLCanvasElement;
  private readonly context: CanvasRenderingContext2D;
  private readonly zoomBehavior: ZoomBehavior<HTMLCanvasElement, unknown>;
  private readonly zoomSelection: Selection<
    HTMLCanvasElement,
    unknown,
    null,
    undefined
  >;
  private nodes: GraphCanvasNode[];
  private edgeGroups: GraphCanvasEdge[][] = [];
  private adjacency: number[][] = [];
  private palette: GraphCanvasPalette;
  private readonly reduceMotion: boolean;
  private readonly onPin: GraphCanvasRendererOptions['onPin'];
  private readonly onRelease: GraphCanvasRendererOptions['onRelease'];
  private readonly onOpen: GraphCanvasRendererOptions['onOpen'];
  private positions: Float32Array;
  private width = 1;
  private height = 1;
  private camera: Camera = { x: 0, y: 0, zoom: 1 };
  private targetCamera: Camera = { x: 0, y: 0, zoom: 1 };
  private animationFrame: number | null = null;
  private lastFrameTime = 0;
  private hoveredNode = -1;
  private previousHoveredNode = -1;
  private hoverDepths: Map<number, number> | null = null;
  private previousHoverDepths: Map<number, number> | null = null;
  private hoverTransition = 1;
  private pointerId: number | null = null;
  private interaction: PointerInteraction | null = null;
  private clickCandidate: ClickCandidate | null = null;
  private lastPointerGestureDragged = false;
  private applyingCameraAnimation = false;
  private cameraGestureActive = false;
  private ready = false;
  private revealing = false;
  private revealStartedAt: number | null = null;
  private revealedNodeCount = 0;
  private destroyed = false;
  private userInteracted = false;

  constructor(options: GraphCanvasRendererOptions) {
    const context = options.canvas.getContext('2d', {
      alpha: false,
      desynchronized: true,
    });
    if (!context) throw new Error('当前浏览器不支持 Canvas 2D。');

    this.canvas = options.canvas;
    this.canvas.dataset.graphLayoutStatus = 'running';
    this.context = context;
    this.nodes = options.nodes;
    this.positions = options.positions;
    this.rebuildTopology(options.edges);
    this.palette = options.palette;
    this.reduceMotion = options.reduceMotion;
    this.onPin = options.onPin;
    this.onRelease = options.onRelease;
    this.onOpen = options.onOpen;
    this.canvas.style.cursor = 'grab';

    this.zoomSelection = select(this.canvas);
    this.zoomBehavior = zoom<HTMLCanvasElement, unknown>()
      .scaleExtent([MIN_ZOOM, MAX_ZOOM])
      .touchable(true)
      .duration(this.reduceMotion ? 0 : 180)
      .filter(this.acceptCameraGesture)
      .on('start.graph', this.handleCameraGestureStart)
      .on('zoom.graph', this.handleCameraGesture)
      .on('end.graph', this.handleCameraGestureEnd);
    this.zoomSelection.call(this.zoomBehavior);
    // 普通滚轮自行处理为指针缩放；触摸、鼠标拖拽和双指捏合仍交给 d3-zoom。
    this.zoomSelection.on('wheel.zoom', null);

    this.canvas.addEventListener('pointerdown', this.handlePointerDown);
    this.canvas.addEventListener('pointermove', this.handlePointerMove);
    this.canvas.addEventListener('pointerup', this.handlePointerUp);
    this.canvas.addEventListener('pointercancel', this.handlePointerUp);
    this.canvas.addEventListener('pointerleave', this.handlePointerLeave);
    this.canvas.addEventListener('dblclick', this.handleDoubleClick);
    this.canvas.addEventListener('wheel', this.handleWheel, { passive: false });
    this.canvas.addEventListener('contextmenu', this.handleContextMenu);
    this.resize();
  }

  private rebuildTopology(edges: GraphCanvasEdge[]) {
    const edgeGroups = new Map<string, GraphCanvasEdge[]>();
    for (const edge of edges) {
      const key = [
        edge.color,
        edge.opacity,
        edge.lineWidth ?? 1,
        edge.dash?.join(',') ?? '',
      ].join('\u0000');
      const group = edgeGroups.get(key);
      if (group) group.push(edge);
      else edgeGroups.set(key, [edge]);
    }
    this.edgeGroups = [...edgeGroups.values()].map((group) =>
      group.sort(
        (left, right) =>
          Math.max(left.source, left.target) -
          Math.max(right.source, right.target),
      ),
    );
    this.adjacency = buildGraphAdjacency(this.nodes.length, edges);
  }

  private refreshHoverNeighborhoods() {
    this.hoverDepths =
      this.hoveredNode >= 0
        ? computeHoverDepths(this.adjacency, this.hoveredNode)
        : null;
    this.previousHoverDepths =
      this.previousHoveredNode >= 0
        ? computeHoverDepths(this.adjacency, this.previousHoveredNode)
        : null;
  }

  destroy() {
    this.destroyed = true;
    if (this.animationFrame !== null) {
      cancelAnimationFrame(this.animationFrame);
      this.animationFrame = null;
    }
    this.zoomSelection.on('.zoom', null);
    this.zoomBehavior.on('.graph', null);
    this.canvas.removeEventListener('pointerdown', this.handlePointerDown);
    this.canvas.removeEventListener('pointermove', this.handlePointerMove);
    this.canvas.removeEventListener('pointerup', this.handlePointerUp);
    this.canvas.removeEventListener('pointercancel', this.handlePointerUp);
    this.canvas.removeEventListener('pointerleave', this.handlePointerLeave);
    this.canvas.removeEventListener('dblclick', this.handleDoubleClick);
    this.canvas.removeEventListener('wheel', this.handleWheel);
    this.canvas.removeEventListener('contextmenu', this.handleContextMenu);
    this.canvas.style.cursor = '';
  }

  resize(fitToGraph = false) {
    const previousWidth = this.width;
    const previousHeight = this.height;
    const bounds = this.canvas.getBoundingClientRect();
    this.width = Math.max(1, bounds.width);
    this.height = Math.max(1, bounds.height);
    const pixelRatio = Math.max(1, window.devicePixelRatio || 1);
    const physicalWidth = Math.round(this.width * pixelRatio);
    const physicalHeight = Math.round(this.height * pixelRatio);
    if (
      this.canvas.width !== physicalWidth ||
      this.canvas.height !== physicalHeight
    ) {
      this.canvas.width = physicalWidth;
      this.canvas.height = physicalHeight;
    }
    if (this.ready) {
      if (fitToGraph) {
        this.fitGraph(true);
      } else {
        const differenceX = (this.width - previousWidth) / 2;
        const differenceY = (this.height - previousHeight) / 2;
        this.camera.x += differenceX;
        this.camera.y += differenceY;
        this.targetCamera.x += differenceX;
        this.targetCamera.y += differenceY;
        this.applyZoomTransform(this.camera, true);
      }
    }
    this.draw();
  }

  updateGraph(
    nodes: GraphCanvasNode[],
    edges: GraphCanvasEdge[],
    positions: Float32Array,
  ) {
    this.positions = positions;
    this.replaceGraphContent(nodes, edges);
  }

  /** 只替换节点与连线的颜色，保留布局、相机和悬停状态。 */
  updateColors(nodes: GraphCanvasNode[], edges: GraphCanvasEdge[]) {
    this.replaceGraphContent(nodes, edges);
  }

  setPalette(palette: GraphCanvasPalette) {
    this.palette = palette;
    this.requestRender();
  }

  private replaceGraphContent(
    nodes: GraphCanvasNode[],
    edges: GraphCanvasEdge[],
  ) {
    const hoveredId = this.nodes[this.hoveredNode]?.id;
    const previousHoveredId = this.nodes[this.previousHoveredNode]?.id;
    const interactingId = this.interaction
      ? this.nodes[this.interaction.index]?.id
      : undefined;

    this.nodes = nodes;
    if (!this.revealing) this.revealedNodeCount = nodes.length;
    this.rebuildTopology(edges);
    const nodeIndexes = new Map(nodes.map((node, index) => [node.id, index]));
    this.hoveredNode = hoveredId ? (nodeIndexes.get(hoveredId) ?? -1) : -1;
    this.previousHoveredNode = previousHoveredId
      ? (nodeIndexes.get(previousHoveredId) ?? -1)
      : -1;
    this.refreshHoverNeighborhoods();
    if (this.interaction) {
      const index = interactingId ? nodeIndexes.get(interactingId) : undefined;
      if (index === undefined) this.interaction = null;
      else this.interaction.index = index;
    }
    this.requestRender();
  }

  /**
   * Initial layout frames may drift around the simulation origin. Compensating
   * that drift keeps the fitted graph anchored while its internal shape settles.
   */
  setPositions(positions: Float32Array, preserveGraphCenter = false) {
    const previousCenter =
      preserveGraphCenter && this.ready && !this.userInteracted
        ? graphBoundsCenter(this.positions)
        : null;
    if (this.interaction?.dragging) {
      const index = this.interaction.index;
      positions[index * 2] = this.positions[index * 2] ?? 0;
      positions[index * 2 + 1] = this.positions[index * 2 + 1] ?? 0;
    }
    this.positions = positions;
    const nextCenter = previousCenter
      ? graphBoundsCenter(this.positions)
      : null;
    if (previousCenter && nextCenter) {
      this.camera = compensateCameraForGraphCenter(
        this.camera,
        previousCenter,
        nextCenter,
      );
      this.targetCamera = compensateCameraForGraphCenter(
        this.targetCamera,
        previousCenter,
        nextCenter,
      );
      this.applyZoomTransform(this.camera, true);
    }
    this.requestRender();
  }

  setReady(revealGraph = false) {
    this.ready = true;
    if (revealGraph && !this.reduceMotion && this.nodes.length > 0) {
      this.revealing = true;
      this.revealStartedAt = null;
      this.revealedNodeCount = 0;
    } else {
      this.revealing = false;
      this.revealedNodeCount = this.nodes.length;
    }
    this.requestRender();
  }

  setLayoutSettled(settled: boolean) {
    this.canvas.dataset.graphLayoutStatus = settled ? 'settled' : 'running';
  }

  fitGraph(immediate = false) {
    if (this.nodes.length === 0) return;
    let minimumX = Number.POSITIVE_INFINITY;
    let maximumX = Number.NEGATIVE_INFINITY;
    let minimumY = Number.POSITIVE_INFINITY;
    let maximumY = Number.NEGATIVE_INFINITY;

    for (let index = 0; index < this.nodes.length; index += 1) {
      const x = this.positions[index * 2] ?? 0;
      const y = this.positions[index * 2 + 1] ?? 0;
      minimumX = Math.min(minimumX, x);
      maximumX = Math.max(maximumX, x);
      minimumY = Math.min(minimumY, y);
      maximumY = Math.max(maximumY, y);
    }

    const graphWidth = Math.max(1, maximumX - minimumX);
    const graphHeight = Math.max(1, maximumY - minimumY);
    const zoom = clamp(
      Math.min(
        Math.max(1, this.width - FIT_PADDING * 2) / graphWidth,
        Math.max(1, this.height - FIT_PADDING * 2) / graphHeight,
      ),
      MIN_ZOOM,
      MAX_ZOOM,
    );
    const centerX = (minimumX + maximumX) / 2;
    const centerY = (minimumY + maximumY) / 2;
    this.moveCamera(
      {
        zoom,
        x: this.width / 2 - centerX * zoom,
        y: this.height / 2 - centerY * zoom,
      },
      immediate,
    );
  }

  fitIfUntouched(immediate = false) {
    if (!this.userInteracted) this.fitGraph(immediate);
  }

  private moveCamera(camera: Camera, immediate: boolean) {
    this.targetCamera = camera;
    if (immediate || this.reduceMotion) {
      this.applyZoomTransform(camera);
    } else {
      this.requestRender();
    }
  }

  private zoomAt(screenX: number, screenY: number, factor: number) {
    this.zoomSelection.call(this.zoomBehavior.scaleBy, factor, [
      screenX,
      screenY,
    ]);
  }

  private applyZoomTransform(camera: Camera, preserveTarget = false) {
    this.applyingCameraAnimation = preserveTarget;
    this.zoomSelection.call(
      this.zoomBehavior.transform,
      zoomIdentity.translate(camera.x, camera.y).scale(camera.zoom),
    );
    this.applyingCameraAnimation = false;
  }

  private requestRender() {
    if (this.destroyed || this.animationFrame !== null) return;
    this.animationFrame = requestAnimationFrame(this.renderFrame);
  }

  private readonly renderFrame = (time: number) => {
    this.animationFrame = null;
    const elapsed = this.lastFrameTime === 0 ? 16 : time - this.lastFrameTime;
    this.lastFrameTime = time;

    const differenceX = this.targetCamera.x - this.camera.x;
    const differenceY = this.targetCamera.y - this.camera.y;
    const differenceZoom = this.targetCamera.zoom - this.camera.zoom;
    const cameraMoving =
      Math.abs(differenceX) > POSITION_EPSILON ||
      Math.abs(differenceY) > POSITION_EPSILON ||
      Math.abs(differenceZoom) > ZOOM_EPSILON;
    const hoverMoving =
      !this.reduceMotion && this.hoverTransition < 1 - HOVER_EPSILON;
    let revealMoving = false;
    if (this.revealing) {
      this.revealStartedAt ??= time;
      this.revealedNodeCount = graphRevealNodeCount(
        this.nodes.length,
        time - this.revealStartedAt,
      );
      revealMoving = this.revealedNodeCount < this.nodes.length;
      if (!revealMoving) {
        this.revealing = false;
        this.revealStartedAt = null;
      }
    }

    if (cameraMoving && !this.reduceMotion) {
      const amount = 1 - Math.exp(-Math.min(48, elapsed) / CAMERA_RESPONSE_MS);
      this.applyZoomTransform(
        {
          x: this.camera.x + differenceX * amount,
          y: this.camera.y + differenceY * amount,
          zoom: this.camera.zoom + differenceZoom * amount,
        },
        true,
      );
    } else {
      if (cameraMoving) this.applyZoomTransform(this.targetCamera, true);
    }
    if (hoverMoving) {
      this.hoverTransition = Math.min(
        1,
        this.hoverTransition + elapsed / HOVER_TRANSITION_MS,
      );
    } else {
      this.hoverTransition = 1;
    }

    this.draw();
    if ((cameraMoving || hoverMoving || revealMoving) && !this.reduceMotion) {
      this.requestRender();
    }
  };

  private draw() {
    const context = this.context;
    const pixelRatio = Math.max(1, window.devicePixelRatio || 1);
    context.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0);
    context.fillStyle = this.palette.background;
    context.fillRect(0, 0, this.width, this.height);
    context.lineCap = 'round';
    if (!this.ready) return;
    const transition = this.reduceMotion
      ? 1
      : 1 - (1 - this.hoverTransition) ** 3;
    const hoverAmount = this.getHoverAmount(transition);
    const visibleNodeCount = Math.min(
      this.nodes.length,
      this.revealedNodeCount,
    );

    for (const group of this.edgeGroups) {
      const first = group[0];
      if (!first) continue;
      if (hoverAmount <= 0) {
        context.beginPath();
        this.traceEdges(group, visibleNodeCount);
        context.globalAlpha = first.opacity;
        context.strokeStyle = first.color;
        context.lineWidth = first.lineWidth ?? 1;
        context.setLineDash(first.dash ?? []);
        context.stroke();
        continue;
      }
      for (const edge of group) {
        if (
          edge.source >= visibleNodeCount ||
          edge.target >= visibleNodeCount
        ) {
          break;
        }
        const emphasis = this.getEdgeEmphasis(edge, transition);
        const peakOpacity = edge.hoverOpacity ?? HOVER_EDGE_PEAK_OPACITY;
        const hoverOpacity =
          MUTED_EDGE_OPACITY + (peakOpacity - MUTED_EDGE_OPACITY) * emphasis;
        context.beginPath();
        this.traceEdges([edge], visibleNodeCount);
        context.globalAlpha =
          edge.opacity + (hoverOpacity - edge.opacity) * hoverAmount;
        context.strokeStyle = mixGraphColor(
          edge.color,
          this.palette.mutedEdge,
          hoverMuteForEdge(
            this.previousHoverDepths,
            this.hoverDepths,
            edge.source,
            edge.target,
            transition,
          ),
        );
        context.lineWidth = edge.lineWidth ?? 1;
        context.setLineDash(edge.dash ?? []);
        context.stroke();
      }
    }
    context.setLineDash([]);
    context.globalAlpha = 1;

    const nodeScale = this.camera.zoom;
    for (let index = 0; index < visibleNodeCount; index += 1) {
      const node = this.nodes[index];
      if (!node) continue;
      const x =
        (this.positions[index * 2] ?? 0) * this.camera.zoom + this.camera.x;
      const y =
        (this.positions[index * 2 + 1] ?? 0) * this.camera.zoom + this.camera.y;
      const radius = graphNodeScreenRadius(node.diameter, nodeScale);
      if (
        x + radius < 0 ||
        x - radius > this.width ||
        y + radius < 0 ||
        y - radius > this.height
      ) {
        continue;
      }
      const emphasis = this.getNodeEmphasis(index, transition);
      const nodeAlpha =
        1 - hoverAmount * (1 - emphasis) * (1 - MUTED_NODE_OPACITY);
      const fill = mixGraphColor(
        node.fill,
        this.palette.mutedNode,
        hoverMuteForNode(
          this.previousHoverDepths,
          this.hoverDepths,
          index,
          transition,
        ),
      );
      context.globalAlpha = nodeAlpha;
      context.beginPath();
      context.arc(x, y, radius, 0, Math.PI * 2);
      context.fillStyle = createGraphNodeFill(context, x, y, radius, fill);
      context.fill();
    }
    context.globalAlpha = 1;

    const labelAlpha = clamp(
      (this.camera.zoom - LABEL_FADE_START) /
        (LABEL_FADE_END - LABEL_FADE_START),
      0,
      1,
    );
    if (labelAlpha > 0) {
      for (let index = 0; index < visibleNodeCount; index += 1) {
        this.drawLabel(index, nodeScale, hoverAmount, transition, labelAlpha);
      }
    }
  }

  private traceEdges(edges: GraphCanvasEdge[], visibleNodeCount: number) {
    for (const edge of edges) {
      if (edge.source >= visibleNodeCount || edge.target >= visibleNodeCount) {
        break;
      }
      const sourceX =
        (this.positions[edge.source * 2] ?? 0) * this.camera.zoom +
        this.camera.x;
      const sourceY =
        (this.positions[edge.source * 2 + 1] ?? 0) * this.camera.zoom +
        this.camera.y;
      const targetX =
        (this.positions[edge.target * 2] ?? 0) * this.camera.zoom +
        this.camera.x;
      const targetY =
        (this.positions[edge.target * 2 + 1] ?? 0) * this.camera.zoom +
        this.camera.y;
      if (
        Math.max(sourceX, targetX) < 0 ||
        Math.min(sourceX, targetX) > this.width ||
        Math.max(sourceY, targetY) < 0 ||
        Math.min(sourceY, targetY) > this.height
      ) {
        continue;
      }
      this.context.moveTo(sourceX, sourceY);
      this.context.lineTo(targetX, targetY);
    }
  }

  private drawLabel(
    index: number,
    nodeScale: number,
    hoverAmount: number,
    transition: number,
    labelAlpha: number,
  ) {
    const node = this.nodes[index];
    if (!node) return;
    const x =
      (this.positions[index * 2] ?? 0) * this.camera.zoom + this.camera.x;
    const y =
      (this.positions[index * 2 + 1] ?? 0) * this.camera.zoom + this.camera.y;
    const radius = graphNodeScreenRadius(node.diameter, nodeScale);
    if (x < -140 || x > this.width + 140 || y < -24 || y > this.height + 24) {
      return;
    }

    const fontSize = 11;
    const context = this.context;
    context.font = `500 ${fontSize}px system-ui, sans-serif`;
    context.textAlign = 'center';
    context.textBaseline = 'top';
    const emphasis = this.getNodeEmphasis(index, transition);
    context.globalAlpha =
      labelAlpha *
      (1 - hoverAmount * (1 - emphasis) * (1 - MUTED_LABEL_OPACITY));
    context.fillStyle = node.labelColor;
    const label = fitGraphLabel(
      node.label,
      LABEL_MAX_WIDTH,
      (value) => context.measureText(value).width,
    );
    context.fillText(label, x, y + radius + 5);
    context.globalAlpha = 1;
  }

  private hitTest(screenX: number, screenY: number) {
    if (!this.ready) return -1;
    const nodeScale = this.camera.zoom;
    let closestIndex = -1;
    let closestDistance = Number.POSITIVE_INFINITY;
    const visibleNodeCount = Math.min(
      this.nodes.length,
      this.revealedNodeCount,
    );
    for (let index = 0; index < visibleNodeCount; index += 1) {
      const node = this.nodes[index];
      if (!node) continue;
      const x =
        (this.positions[index * 2] ?? 0) * this.camera.zoom + this.camera.x;
      const y =
        (this.positions[index * 2 + 1] ?? 0) * this.camera.zoom + this.camera.y;
      const distance = Math.hypot(screenX - x, screenY - y);
      const radius = Math.max(
        5,
        graphNodeScreenRadius(node.diameter, nodeScale),
      );
      if (distance <= radius && distance < closestDistance) {
        closestIndex = index;
        closestDistance = distance;
      }
    }
    return closestIndex;
  }

  private getHoverAmount(transition: number) {
    if (this.previousHoveredNode >= 0) {
      return this.hoveredNode >= 0 ? 1 : 1 - transition;
    }
    return this.hoveredNode >= 0 ? transition : 0;
  }

  private getNodeEmphasis(index: number, transition: number) {
    const previous = hoverEmphasisForDepth(
      this.previousHoverDepths?.get(index),
    );
    const current = hoverEmphasisForDepth(this.hoverDepths?.get(index));
    return previous + (current - previous) * transition;
  }

  private getEdgeEmphasis(edge: GraphCanvasEdge, transition: number) {
    const previous = hoverEmphasisForEdge(
      this.previousHoverDepths,
      edge.source,
      edge.target,
    );
    const current = hoverEmphasisForEdge(
      this.hoverDepths,
      edge.source,
      edge.target,
    );
    return previous + (current - previous) * transition;
  }

  private setHoveredNode(index: number) {
    if (index === this.hoveredNode) return;
    this.previousHoveredNode = this.hoveredNode;
    this.previousHoverDepths = this.hoverDepths;
    this.hoveredNode = index;
    this.hoverDepths =
      index >= 0 ? computeHoverDepths(this.adjacency, index) : null;
    this.hoverTransition = this.reduceMotion ? 1 : 0;
    if (!this.cameraGestureActive && !this.interaction) {
      this.canvas.style.cursor = this.nodes[index]?.openable
        ? 'pointer'
        : 'grab';
    }
    this.requestRender();
  }

  private pointerPosition(event: MouseEvent) {
    const bounds = this.canvas.getBoundingClientRect();
    return {
      x: event.clientX - bounds.left,
      y: event.clientY - bounds.top,
    };
  }

  private readonly acceptCameraGesture = (
    event: MouseEvent | TouchEvent | WheelEvent,
  ) => {
    if (event.type === 'wheel') return false;
    if (!(event instanceof MouseEvent)) return true;
    if (event.ctrlKey || (event.button !== 0 && event.button !== 1)) {
      return false;
    }
    const pointer = this.pointerPosition(event);
    return this.hitTest(pointer.x, pointer.y) < 0;
  };

  private readonly handleCameraGestureStart = (
    event: D3ZoomEvent<HTMLCanvasElement, unknown>,
  ) => {
    if (!event.sourceEvent) return;
    this.cameraGestureActive = true;
    this.userInteracted = true;
    this.targetCamera = { ...this.camera };
    this.setHoveredNode(-1);
    this.canvas.style.cursor = 'grabbing';
  };

  private readonly handleCameraGesture = (
    event: D3ZoomEvent<HTMLCanvasElement, unknown>,
  ) => {
    this.camera = {
      x: event.transform.x,
      y: event.transform.y,
      zoom: event.transform.k,
    };
    if (!this.applyingCameraAnimation) {
      this.targetCamera = { ...this.camera };
    }
    this.requestRender();
  };

  private readonly handleCameraGestureEnd = (
    event: D3ZoomEvent<HTMLCanvasElement, unknown>,
  ) => {
    const source = event.sourceEvent;
    if (!source) return;
    this.cameraGestureActive = false;
    if (!(source instanceof MouseEvent)) return;
    const pointer = this.pointerPosition(source);
    this.setHoveredNode(this.hitTest(pointer.x, pointer.y));
  };

  private readonly handlePointerDown = (event: PointerEvent) => {
    if (event.button !== 0) return;
    const pointer = this.pointerPosition(event);
    const nodeIndex = this.hitTest(pointer.x, pointer.y);
    if (nodeIndex < 0) return;
    const node = this.nodes[nodeIndex];
    if (!node) return;
    this.clickCandidate = {
      moved: false,
      pointerId: event.pointerId,
      startX: pointer.x,
      startY: pointer.y,
    };
    this.lastPointerGestureDragged = false;
    if (event.pointerType === 'touch') return;
    this.userInteracted = true;
    this.targetCamera = { ...this.camera };
    this.pointerId = event.pointerId;
    this.canvas.setPointerCapture(event.pointerId);
    const graphX = (pointer.x - this.camera.x) / this.camera.zoom;
    const graphY = (pointer.y - this.camera.y) / this.camera.zoom;
    this.interaction = {
      dragging: false,
      index: nodeIndex,
      offsetX: graphX - (this.positions[nodeIndex * 2] ?? 0),
      offsetY: graphY - (this.positions[nodeIndex * 2 + 1] ?? 0),
    };
    event.preventDefault();
  };

  private readonly handlePointerMove = (event: PointerEvent) => {
    const pointer = this.pointerPosition(event);
    if (
      this.clickCandidate?.pointerId === event.pointerId &&
      Math.hypot(
        pointer.x - this.clickCandidate.startX,
        pointer.y - this.clickCandidate.startY,
      ) > CLICK_MOVE_TOLERANCE
    ) {
      this.clickCandidate.moved = true;
    }
    if (event.pointerType === 'touch' || this.cameraGestureActive) return;
    if (this.pointerId === event.pointerId && this.interaction) {
      if (!this.clickCandidate?.moved) return;
      this.interaction.dragging = true;
      this.canvas.style.cursor = 'grabbing';
      const graphX =
        (pointer.x - this.camera.x) / this.camera.zoom -
        this.interaction.offsetX;
      const graphY =
        (pointer.y - this.camera.y) / this.camera.zoom -
        this.interaction.offsetY;
      const index = this.interaction.index;
      this.positions[index * 2] = graphX;
      this.positions[index * 2 + 1] = graphY;
      const node = this.nodes[index];
      if (node) this.onPin(node.id, graphX, graphY);
      this.requestRender();
      event.preventDefault();
      return;
    }

    const hoveredNode = this.hitTest(pointer.x, pointer.y);
    this.setHoveredNode(hoveredNode);
  };

  private readonly handlePointerUp = (event: PointerEvent) => {
    this.clickCandidate = null;
    if (this.pointerId !== event.pointerId) return;
    this.lastPointerGestureDragged = Boolean(this.interaction?.dragging);
    if (this.interaction?.dragging) {
      const node = this.nodes[this.interaction.index];
      if (node) this.onRelease(node.id);
    }
    this.interaction = null;
    this.pointerId = null;
    if (this.canvas.hasPointerCapture(event.pointerId)) {
      this.canvas.releasePointerCapture(event.pointerId);
    }
    const pointer = this.pointerPosition(event);
    this.setHoveredNode(this.hitTest(pointer.x, pointer.y));
  };

  private readonly handleDoubleClick = (event: MouseEvent) => {
    if (this.lastPointerGestureDragged) return;
    const pointer = this.pointerPosition(event);
    const node = this.nodes[this.hitTest(pointer.x, pointer.y)];
    if (!node?.openable) return;
    this.onOpen(node.id);
    event.preventDefault();
  };

  private readonly handlePointerLeave = () => {
    if (this.interaction) return;
    this.setHoveredNode(-1);
  };

  private readonly handleWheel = (event: WheelEvent) => {
    this.userInteracted = true;
    const { delta, source } = graphWheelInteraction(event, this.height);
    const pointer = this.pointerPosition(event);
    this.zoomAt(
      pointer.x,
      pointer.y,
      Math.exp(
        (-delta * Math.log(1.5)) /
          (source === 'pinch'
            ? TRACKPAD_ZOOM_DISTANCE
            : MOUSE_WHEEL_ZOOM_DISTANCE),
      ),
    );
    event.preventDefault();
  };

  private readonly handleContextMenu = (event: MouseEvent) => {
    event.preventDefault();
  };
}
