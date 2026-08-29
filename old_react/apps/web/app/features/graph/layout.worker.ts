/// <reference lib="webworker" />

import {
  forceCollide,
  forceLink,
  forceManyBody,
  forceSimulation,
  forceX,
  forceY,
  type Simulation,
  type SimulationLinkDatum,
  type SimulationNodeDatum,
} from 'd3-force';
import { GRAPH_FORCE_DEFAULTS } from './layout';
import type {
  GraphLayoutLinkInput,
  GraphLayoutNodeInput,
  GraphLayoutWorkerRequest,
  GraphLayoutWorkerResponse,
} from './layout-worker';
import {
  graphLayoutOutputInterval,
  positionNewGraphNode,
  resolveGraphLayoutLinks,
} from './layout-worker';

interface WorkerNode extends SimulationNodeDatum {
  id: string;
}

interface WorkerLink extends SimulationLinkDatum<WorkerNode> {
  distance?: number;
  strength: number;
}

const SIMULATION_TICK_COUNT = 300;
const INITIAL_TICK_COUNT = 80;
const UPDATE_ALPHA = 0.35;
const workerScope = self as DedicatedWorkerGlobalScope;

let nodes: WorkerNode[] = [];
let nodeById = new Map<string, WorkerNode>();
let simulation: Simulation<WorkerNode, WorkerLink> | null = null;
let linkForce: ReturnType<typeof forceLink<WorkerNode, WorkerLink>> | null =
  null;
let lastOutputTime = 0;
let outputInterval = 1_000 / 60;
let reduceMotion = false;
let revision = 0;

function sendPositions(settled: boolean) {
  const positions = new Float32Array(nodes.length * 2);
  for (const [index, node] of nodes.entries()) {
    positions[index * 2] = node.x ?? 0;
    positions[index * 2 + 1] = node.y ?? 0;
  }
  const response: GraphLayoutWorkerResponse = {
    type: 'positions',
    revision,
    positions: positions.buffer,
    settled,
  };
  workerScope.postMessage(response, [positions.buffer]);
}

function sendCurrentPositions() {
  const now = performance.now();
  if (now - lastOutputTime < outputInterval) return;
  lastOutputTime = now;
  sendPositions(false);
}

function initialize(
  nodeInputs: GraphLayoutWorkerRequest & { type: 'initialize' },
) {
  simulation?.stop();
  revision = nodeInputs.revision;
  reduceMotion = nodeInputs.reduceMotion;
  outputInterval = graphLayoutOutputInterval(
    nodeInputs.nodes.length,
    nodeInputs.links.length,
  );
  nodes = nodeInputs.nodes.map((node) => ({ ...node }));
  nodeById = new Map(nodes.map((node) => [node.id, node]));
  const links: WorkerLink[] = resolveGraphLayoutLinks(nodeInputs.links).map(
    (link) => ({
      distance: link.distance,
      source: link.source,
      strength: link.strength,
      target: link.target,
    }),
  );
  linkForce = forceLink<WorkerNode, WorkerLink>(links)
    .id((node) => node.id)
    .distance((link) => link.distance ?? GRAPH_FORCE_DEFAULTS.linkDistance)
    .strength((link) => link.strength);

  simulation = forceSimulation(nodes)
    .alpha(1)
    .alphaDecay(1 - 0.001 ** (1 / SIMULATION_TICK_COUNT))
    .velocityDecay(0.4)
    // Force order is significant: run collision last so no later force can
    // reintroduce overlap during the same frame. This mirrors Obsidian.
    .force(
      'center-x',
      forceX<WorkerNode>(0).strength(GRAPH_FORCE_DEFAULTS.centerStrength),
    )
    .force(
      'center-y',
      forceY<WorkerNode>(0).strength(GRAPH_FORCE_DEFAULTS.centerStrength),
    )
    .force('link', linkForce)
    .force(
      'charge',
      forceManyBody<WorkerNode>()
        .strength(GRAPH_FORCE_DEFAULTS.repelStrength)
        .distanceMin(30),
    )
    .force(
      'collide',
      forceCollide<WorkerNode>()
        .radius(GRAPH_FORCE_DEFAULTS.collisionRadius)
        .strength(GRAPH_FORCE_DEFAULTS.collisionStrength),
    )
    .stop();

  if (reduceMotion) {
    simulation.tick(SIMULATION_TICK_COUNT);
    sendPositions(true);
    return;
  }

  simulation.tick(INITIAL_TICK_COUNT);
  sendPositions(false);
  simulation
    .on('tick', sendCurrentPositions)
    .on('end', () => sendPositions(true))
    .restart();
}

function reconcileNodes(
  nodeInputs: GraphLayoutNodeInput[],
  links: GraphLayoutLinkInput[],
) {
  const previousPositions = new Map(
    nodes.map((node) => [
      node.id,
      {
        x: node.x ?? 0,
        y: node.y ?? 0,
      },
    ]),
  );
  return nodeInputs.map((node) => {
    const current = nodeById.get(node.id);
    if (current) return current;
    return {
      id: node.id,
      ...positionNewGraphNode(node, links, previousPositions),
    };
  });
}

function updateGraph(
  nodeInputs: GraphLayoutWorkerRequest & { type: 'update' },
) {
  if (!simulation || !linkForce) return;
  simulation.stop();
  revision = nodeInputs.revision;
  outputInterval = graphLayoutOutputInterval(
    nodeInputs.nodes.length,
    nodeInputs.links.length,
  );
  nodes = reconcileNodes(nodeInputs.nodes, nodeInputs.links);
  nodeById = new Map(nodes.map((node) => [node.id, node]));
  const links: WorkerLink[] = resolveGraphLayoutLinks(nodeInputs.links).map(
    (link) => ({
      distance: link.distance,
      source: link.source,
      strength: link.strength,
      target: link.target,
    }),
  );
  simulation.nodes(nodes);
  linkForce.links(links);
  simulation.alpha(Math.max(UPDATE_ALPHA, simulation.alpha())).alphaTarget(0);
  lastOutputTime = 0;

  if (reduceMotion) {
    simulation.tick(SIMULATION_TICK_COUNT);
    sendPositions(true);
    return;
  }

  simulation.tick();
  sendPositions(false);
  simulation.restart();
}

workerScope.onmessage = (event: MessageEvent<GraphLayoutWorkerRequest>) => {
  const message = event.data;
  if (message.type === 'initialize') {
    initialize(message);
    return;
  }
  if (message.type === 'update') {
    updateGraph(message);
    return;
  }

  const node = nodeById.get(message.id);
  if (!node || !simulation) return;
  if (message.type === 'pin') {
    node.fx = message.x;
    node.fy = message.y;
    simulation.alpha(Math.max(0.4, simulation.alpha()));
    simulation.alphaTarget(0.25).restart();
    return;
  }

  node.fx = undefined;
  node.fy = undefined;
  simulation.alphaTarget(0);
};
