import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  Background,
  BackgroundVariant,
  MiniMap,
  ReactFlow,
  useReactFlow,
  type Connection,
  type Edge,
  type IsValidConnection,
  type Node,
  type EdgeChange,
  type NodeChange,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import {
  GRID,
  accepts,
  convertible,
  kind as lookupKind,
  refusal,
  type PortType,
} from '@cyberloom/graph-core';
import { Button } from '@cyberloom/ui';
import { BlockNode, type BlockNodeData } from './BlockNode';
import { LoopFrame, type LoopFrameData } from './LoopFrame';
import { Wire, type WireData } from './Wire';
import { ZoomPill } from './ZoomPill';
import { blockOfKind, portTypeOf, useDocument } from '../stores/document';
import { useRun } from '../stores/run';
import s from './Canvas.module.css';

const nodeTypes = { block: BlockNode, frame: LoopFrame };
const edgeTypes = { wire: Wire };

/** The minimap's box. See the note where it is passed. */
const MINIMAP = { width: 138, height: 88 };

/** What is being dragged, so every port can decide whether to dim. */
export interface DragState {
  node: string;
  port: string;
  side: 'in' | 'out';
  type: PortType;
}

/**
 * The canvas.
 *
 * xyflow owns pan, zoom, selection and hit testing; the document store owns the
 * graph. Every edit is applied to the store, never to xyflow's copy of the
 * nodes, so undo and autosave see it and the canvas stays a view of the file
 * rather than a second source of truth.
 */
export function Canvas() {
  const graph = useDocument((d) => d.graph);
  const selection = useDocument((d) => d.selection);
  const { screenToFlowPosition } = useReactFlow();
  const [drag, setDrag] = useState<DragState | null>(null);
  /** A wire that nearly fit, waiting on an answer about a Convert (§15.5). */
  const [offer, setOffer] = useState<Offer | null>(null);
  /**
   * How big each node turned out to be.
   *
   * A block sizes itself from its content, so only the browser knows the
   * answer; xyflow measures it and reports a `dimensions` change. Because the
   * nodes are rebuilt from the graph on every render, that measurement would
   * be thrown away each time — and anything reading the node objects rather
   * than xyflow's own copy, the minimap above all, would see nodes of no size
   * and draw nothing. Keeping the measurements here hands them back.
   */
  const [measured, setMeasured] = useState<Map<string, { width: number; height: number }>>(
    () => new Map(),
  );

  const wired = useMemo(() => {
    const map = new Map<string, Set<string>>();
    for (const w of graph.wires) {
      for (const end of [w.from, w.to]) {
        if (!map.has(end.node)) map.set(end.node, new Set());
        map.get(end.node)!.add(end.port);
      }
    }
    return map;
  }, [graph.wires]);

  /** Which ids are frames, so a selection knows what it is looking at. */
  const graphNodeKind = useMemo(() => {
    const map = new Map<string, 'block' | 'frame'>();
    for (const b of graph.blocks) map.set(b.id, 'block');
    for (const f of graph.frames) map.set(f.id, 'frame');
    return map;
  }, [graph.blocks, graph.frames]);

  const selectedIds = useMemo(
    () =>
      new Set(
        selection.kind === 'block'
          ? selection.ids
          : selection.kind === 'frame'
            ? [selection.id]
            : [],
      ),
    [selection],
  );

  const running = useRun((r) => r.blocks);

  const nodes = useMemo<Node<BlockNodeData | LoopFrameData>[]>(() => {
    const frames: Node<LoopFrameData>[] = graph.frames.map((frame) => ({
      id: frame.id,
      type: 'frame',
      position: frame.position,
      data: { frame },
      selected: selectedIds.has(frame.id),
      zIndex: -1,
    }));
    const blocks: Node<BlockNodeData>[] = graph.blocks.map((block) => {
      // A block reporting live figures draws them below itself, over whatever
      // is there. The z-index has to be on the node rather than in the CSS:
      // xyflow gives every node its own stacking context, so a rule inside one
      // cannot lift it above its neighbour.
      const live = running[block.id];
      const reporting =
        !!live && (live.state === 'running' || !!live.figure || !!live.error || !!live.output);
      return {
        id: block.id,
        type: 'block',
        position: block.position,
        data: { block, wired: wired.get(block.id) ?? new Set(), drag },
        selected: selectedIds.has(block.id),
        ...(reporting ? { zIndex: 10 } : {}),
      };
    });
    return [...frames, ...blocks].map((node) => {
      const size = measured.get(node.id);
      return size ? { ...node, measured: size } : node;
    });
  }, [graph.frames, graph.blocks, wired, selectedIds, drag, measured, running]);

  const active = useRun((r) => r.active);
  const activeWires = useMemo(() => new Set(active), [active]);

  const edges = useMemo<Edge<WireData>[]>(
    () =>
      graph.wires.map((wire) => ({
        id: wire.id,
        type: 'wire',
        source: wire.from.node,
        sourceHandle: wire.from.port,
        target: wire.to.node,
        targetHandle: wire.to.port,
        selected: selection.kind === 'wire' && selection.id === wire.id,
        data: {
          type: portTypeOf(graph, wire.from.node, wire.from.port, 'out') ?? 'any',
          live: activeWires.has(wire.id),
        },
      })),
    [graph, selection, activeWires],
  );

  /**
   * xyflow reports every change; three of them matter.
   *
   * `select` has to be applied by hand because the nodes are controlled: their
   * `selected` flag comes from the store, so if the store is not told, nothing
   * ever appears selected and the inspector never reshapes.
   *
   * `dimensions` carries the measured size of a block, which is kept because
   * the nodes are rebuilt from the graph and would otherwise lose it.
   *
   * `position` is applied once, when the drag finishes, which makes a move one
   * undo step rather than one per animation frame.
   */
  const onNodesChange = useCallback(
    (changes: NodeChange[]) => {
      const store = useDocument.getState();
      const selects = changes.filter(
        (c): c is NodeChange & { type: 'select'; id: string; selected: boolean } =>
          c.type === 'select',
      );
      if (selects.length > 0) {
        const next = new Set(selectedIds);
        for (const change of selects) {
          if (change.selected) next.add(change.id);
          else next.delete(change.id);
        }
        applySelection(store, graphNodeKind, next);
      }

      const sized = changes.filter((c) => c.type === 'dimensions' && c.dimensions);
      if (sized.length > 0) {
        setMeasured((before) => {
          let next: Map<string, { width: number; height: number }> | null = null;
          for (const change of sized) {
            if (change.type !== 'dimensions' || !change.dimensions) continue;
            const was = before.get(change.id);
            if (was?.width === change.dimensions.width && was.height === change.dimensions.height) {
              continue;
            }
            next ??= new Map(before);
            next.set(change.id, change.dimensions);
          }
          // A new Map only when something actually changed, so a re-measure
          // that agrees with the last one does not re-render the canvas.
          return next ?? before;
        });
      }

      for (const change of changes) {
        if (change.type !== 'position' || change.dragging !== false) continue;
        const node = nodes.find((n) => n.id === change.id);
        const moved = change.position;
        if (!node || !moved) continue;
        const dx = moved.x - node.position.x;
        const dy = moved.y - node.position.y;
        if (dx === 0 && dy === 0) continue;
        // Dragging one block of a multiple selection moves all of them.
        const ids =
          selectedIds.has(change.id) && selectedIds.size > 1 ? [...selectedIds] : [change.id];
        store.moveBlocks(ids, dx, dy);
      }
    },
    [nodes, selectedIds, graphNodeKind],
  );

  /** Selecting a wire replaces whatever was selected: a wire and a block are
   *  different panels, so they cannot both be showing. */
  const onEdgesChange = useCallback((changes: EdgeChange[]) => {
    const store = useDocument.getState();
    for (const change of changes) {
      if (change.type === 'select' && change.selected) {
        store.select({ kind: 'wire', id: change.id });
      }
    }
  }, []);

  const onConnect = useCallback((connection: Connection) => {
    if (!connection.sourceHandle || !connection.targetHandle) return;
    const from = { node: connection.source, port: connection.sourceHandle };
    const to = { node: connection.target, port: connection.targetHandle };
    const store = useDocument.getState();
    if (store.connect(from, to)) return;

    // The types do not match but they are convertible, so offer the block that
    // makes one into the other (SPEC §15.5). Offered rather than inserted: a
    // conversion is always something you can see, and something you agreed to.
    const fromType = portTypeOf(store.graph, from.node, from.port, 'out');
    const toType = portTypeOf(store.graph, to.node, to.port, 'in');
    if (fromType && toType && convertible(fromType, toType)) {
      setOffer({ from, to, fromType, toType });
    }
  }, []);

  /**
   * Whether the wire being dragged may land here. xyflow asks per candidate,
   * which is what dims the ports that cannot accept it: the grammar refuses
   * before the wire exists rather than after (SPEC §4.1).
   */
  const isValidConnection = useCallback<IsValidConnection>(
    (connection) => {
      if (!connection.sourceHandle || !connection.targetHandle) return false;
      if (connection.source === connection.target) return false;
      const from = portTypeOf(graph, connection.source, connection.sourceHandle, 'out');
      const to = portTypeOf(graph, connection.target, connection.targetHandle, 'in');
      return !!from && !!to && accepts(from, to);
    },
    [graph],
  );

  /** Dropping a library row onto the canvas adds that kind where it landed. */
  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();
      const kindId = event.dataTransfer.getData('application/cyberloom-kind');
      if (!kindId) return;
      const store = useDocument.getState();
      const at = screenToFlowPosition({ x: event.clientX, y: event.clientY });
      store.addBlock(blockOfKind(store.graph, kindId, at));
    },
    [screenToFlowPosition],
  );

  return (
    <div
      className={s.canvas}
      onDrop={onDrop}
      onDragOver={(e) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = 'copy';
      }}
    >
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onConnectStart={(_, params) => {
          if (!params.nodeId || !params.handleId) return;
          const side = params.handleType === 'source' ? 'out' : 'in';
          const type = portTypeOf(graph, params.nodeId, params.handleId, side);
          if (type) setDrag({ node: params.nodeId, port: params.handleId, side, type });
        }}
        onConnectEnd={() => setDrag(null)}
        isValidConnection={isValidConnection}
        snapToGrid
        snapGrid={[GRID, GRID]}
        // xyflow's default multi-select key is Ctrl on Linux and Cmd on macOS.
        // SPEC §2.4 promises Shift-click, so all three are accepted: the
        // platform habit still works and the documented gesture works too.
        multiSelectionKeyCode={['Shift', 'Control', 'Meta']}
        // Delete is handled by the shell's keyboard map, so that one Backspace
        // goes through the store and is undoable.
        deleteKeyCode={null}
        minZoom={0.25}
        maxZoom={2}
        selectionOnDrag
        panOnDrag={[1, 2]}
        proOptions={{ hideAttribution: true }}
        fitView
        fitViewOptions={{ padding: 0.12, maxZoom: 1 }}
      >
        <Background variant={BackgroundVariant.Dots} gap={GRID} size={1} color="var(--grid-dot)" />
        <MiniMap
          className={s.minimap}
          // xyflow reads the width and height off the inline style to work out
          // its own scale, so unlike everything else on the canvas this size
          // cannot live in the stylesheet: given only CSS it draws a 200x150
          // map and the box crops it.
          style={{ width: MINIMAP.width, height: MINIMAP.height }}
          pannable
          zoomable
          maskColor="rgba(8, 9, 11, 0.72)"
          nodeColor={(node) => {
            if (node.type === 'frame') return 'var(--cat-control)';
            const data = node.data as BlockNodeData;
            return `var(--cat-${lookupKind(data.block.kind)?.category ?? 'custom'})`;
          }}
        />
      </ReactFlow>
      <ZoomPill />
      {offer && <ConvertOffer offer={offer} onDone={() => setOffer(null)} />}
      {drag && <DragTooltip drag={drag} />}
    </div>
  );
}

/** Turn a set of selected node ids into the selection the inspector reads. */
function applySelection(
  store: ReturnType<typeof useDocument.getState>,
  kinds: Map<string, 'block' | 'frame'>,
  ids: Set<string>,
) {
  const list = [...ids];
  if (list.length === 0) {
    store.select({ kind: 'none' });
    return;
  }
  if (list.length === 1 && kinds.get(list[0]!) === 'frame') {
    store.select({ kind: 'frame', id: list[0]! });
    return;
  }
  const blocks = list.filter((id) => kinds.get(id) === 'block');
  store.select(blocks.length === 0 ? { kind: 'none' } : { kind: 'block', ids: blocks });
}

/**
 * The tooltip that follows a wire drag. It says what is being carried, and over
 * a port it cannot reach it says why — offering the Convert block where one
 * would help (SPEC §5.5).
 */
function DragTooltip({ drag }: { drag: DragState }) {
  const [at, setAt] = useState({ x: 0, y: 0 });
  const [over, setOver] = useState<string | null>(null);
  const graph = useDocument((d) => d.graph);
  const frame = useRef(0);

  useEffect(() => {
    const move = (e: PointerEvent) => {
      cancelAnimationFrame(frame.current);
      frame.current = requestAnimationFrame(() => {
        setAt({ x: e.clientX, y: e.clientY });
        const port = (e.target as HTMLElement | null)?.closest?.('[data-port]');
        setOver(port?.getAttribute('data-port') ?? null);
      });
    };
    window.addEventListener('pointermove', move);
    return () => {
      window.removeEventListener('pointermove', move);
      cancelAnimationFrame(frame.current);
    };
  }, []);

  let message = `${drag.type} · ${drag.node}.${drag.port}`;
  if (over && over !== `${drag.node}.${drag.port}`) {
    const [node, port] = over.split('.');
    const otherSide = drag.side === 'out' ? 'in' : 'out';
    const other = portTypeOf(graph, node!, port!, otherSide);
    if (other) {
      const [from, to] = drag.side === 'out' ? [drag.type, other] : [other, drag.type];
      message = refusal(from, to) ?? `${from} fits ${to}`;
    }
  }

  return (
    <div className={s.dragTooltip} style={{ left: at.x + 14, top: at.y + 14 }}>
      {message}
    </div>
  );
}

/** A wire the grammar refused, and the Convert that would make it fit. */
interface Offer {
  from: { node: string; port: string };
  to: { node: string; port: string };
  fromType: PortType;
  toType: PortType;
}

/**
 * "The shell offers to insert a Convert block on the wire" (SPEC §15.5).
 *
 * Offered rather than done: a conversion is always a visible block, and a block
 * that appeared because a drag nearly landed is a block nobody put there. One
 * click makes it; anything else leaves the graph as it was.
 */
function ConvertOffer({ offer, onDone }: { offer: Offer; onDone: () => void }) {
  const convertOnWire = useDocument((d) => d.convertOnWire);
  const select = useDocument((d) => d.select);

  return (
    <div className={s.offer} data-testid="convert-offer">
      <div className={s.offerText}>
        <strong>{offer.fromType}</strong> does not fit <strong>{offer.toType}</strong>, but it can
        be converted.
      </div>
      <div className={s.offerRow}>
        <Button
          label="Insert a Convert"
          variant="primary"
          onClick={() => {
            const id = convertOnWire(offer.from, offer.to);
            if (id) select({ kind: 'block', ids: [id] });
            onDone();
          }}
        />
        <Button label="Leave it" onClick={onDone} />
      </div>
    </div>
  );
}
