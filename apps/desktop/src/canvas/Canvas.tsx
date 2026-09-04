import { useEffect, useMemo } from 'react';
import {
  Background,
  BackgroundVariant,
  MiniMap,
  ReactFlow,
  useEdgesState,
  useNodesState,
  type Edge,
  type Node,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import {
  GRID,
  kind as lookupKind,
  type Graph,
  type PortType,
} from '@cyberloom/graph-core';
import { BlockNode, type BlockNodeData } from './BlockNode';
import { LoopFrame, type LoopFrameData } from './LoopFrame';
import { Wire, type WireData } from './Wire';
import { ZoomPill } from './ZoomPill';
import s from './Canvas.module.css';

const nodeTypes = { block: BlockNode, frame: LoopFrame };
const edgeTypes = { wire: Wire };

/**
 * The canvas.
 *
 * xyflow owns pan, zoom, selection and hit testing; everything about how a
 * block or a wire *looks* is ours. This slice is read-only: nothing here can
 * move a block or draw a wire, which is slice 3.
 */
export function Canvas({ graph }: { graph: Graph }) {
  const initialNodes = useMemo<Node<BlockNodeData | LoopFrameData>[]>(() => {
    // Which ports carry a wire, so an unwired optional port stays hidden.
    const wired = new Map<string, Set<string>>();
    for (const w of graph.wires) {
      for (const end of [w.from, w.to]) {
        if (!wired.has(end.node)) wired.set(end.node, new Set());
        wired.get(end.node)!.add(end.port);
      }
    }
    // Frames come first and sit behind: a loop is the region its blocks are
    // in, so it must not cover them.
    const frames: Node<LoopFrameData>[] = graph.frames.map((frame) => ({
      id: frame.id,
      type: 'frame',
      position: frame.position,
      data: { frame },
      draggable: false,
      selectable: true,
      zIndex: -1,
    }));
    const blocks: Node<BlockNodeData>[] = graph.blocks.map((block) => ({
      id: block.id,
      type: 'block',
      position: block.position,
      data: { block, wired: wired.get(block.id) ?? new Set() },
      draggable: false,
      selectable: true,
    }));
    return [...frames, ...blocks];
  }, [graph]);

  const initialEdges = useMemo<Edge<WireData>[]>(
    () =>
      graph.wires.map((wire) => ({
        id: wire.id,
        type: 'wire',
        source: wire.from.node,
        sourceHandle: wire.from.port,
        target: wire.to.node,
        targetHandle: wire.to.port,
        data: { type: portTypeOf(graph, wire.from.node, wire.from.port) },
      })),
    [graph],
  );

  // React Flow has to own the node list, not merely be handed one: it writes
  // each node's measured size back into its own store, and things that read
  // from there — the minimap, fitView, hit testing — see nothing without it.
  // Passing `nodes` with no `onNodesChange` leaves them permanently unmeasured.
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  useEffect(() => setNodes(initialNodes), [initialNodes, setNodes]);
  useEffect(() => setEdges(initialEdges), [initialEdges, setEdges]);

  return (
    <div className={s.canvas}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        defaultViewport={{
          x: graph.ui.viewport.x,
          y: graph.ui.viewport.y,
          zoom: graph.ui.viewport.zoom,
        }}
        minZoom={0.25}
        maxZoom={2}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable
        proOptions={{ hideAttribution: true }}
        fitView
        fitViewOptions={{ padding: 0.12, maxZoom: 1 }}
      >
        <Background variant={BackgroundVariant.Dots} gap={GRID} size={1} color="var(--grid-dot)" />
        <MiniMap
          className={s.minimap}
          pannable
          zoomable
          maskColor="rgba(8, 9, 11, 0.72)"
          nodeColor={(node) => {
            if (node.type === 'frame') return 'var(--cat-control)';
            const data = node.data as BlockNodeData;
            const category = lookupKind(data.block.kind)?.category ?? 'custom';
            return `var(--cat-${category})`;
          }}
        />
      </ReactFlow>
      <ZoomPill />
    </div>
  );
}

/** A wire is coloured by the type of the port it leaves. */
function portTypeOf(graph: Graph, node: string, port: string): PortType {
  const block = graph.blocks.find((b) => b.id === node);
  if (block) {
    if (block.kind === 'custom') {
      return block.ports.find((p) => p.name === port && p.side === 'out')?.type ?? 'any';
    }
    const def = lookupKind(block.kind)?.ports.find((p) => p.name === port && p.side === 'out');
    if (def) return def.type;
  }
  // A loop frame has ports of its own, and they are the Loop kind's.
  const frame = graph.frames.find((f) => f.id === node);
  if (frame) {
    return lookupKind('loop')?.ports.find((p) => p.name === port && p.side === 'out')?.type ?? 'any';
  }
  return 'any';
}
