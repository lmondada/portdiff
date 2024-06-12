import { Node, Edge, XYPosition, NodeChange, applyNodeChanges, ReactFlowInstance, EdgeChange, applyEdgeChanges, Connection } from "reactflow";
import { ExternalNode, InternalNode, PlacedWasmEdge, PlacedWasmNode, WasmEdge, WasmGraph, WasmNode } from "./wasm_api";
import { removeBoundary } from "./boundary";
import { CSSProperties, useCallback, useEffect, useState } from "react";
import { v4 as uuid } from "uuid";

class RFGraph {
    private nodes: PlacedWasmNode[] = [];
    private edges: PlacedWasmEdge[] = [];
    private positions: Record<string, XYPosition> = {};

    constructor(nodes: PlacedWasmNode[], edges: PlacedWasmEdge[], positions: Record<string, XYPosition>) {
        this.nodes = nodes;
        this.edges = edges;
        this.positions = positions;
    }

    static initGraph({ nodes, edges }: WasmGraph, positions: Record<string, XYPosition>) {
        const placedNodes = nodes.map(node => ({
            ...node,
            position: positions[node.id],
            selected: false,
        }));
        const placedEdges = edges.map(edge => ({
            id: edge.id,
            source: edge.source,
            target: edge.target,
            sourceHandle: `out${edge.sourceHandle}`,
            targetHandle: `in${edge.targetHandle}`,
            style: edgeStyle(edge.style),
        }));
        return new RFGraph(placedNodes, placedEdges, positions);
    }

    getNodes(): WasmNode[] {
        return this.nodes.map(node => ({
            id: node.id,
            type: node.type,
            data: node.data
        })) as WasmNode[];
    }

    getEdges(): WasmEdge[] {
        return this.edges.map(edge => ({
            id: edge.id,
            source: edge.source,
            target: edge.target,
            sourceHandle: parseInt(edge.sourceHandle?.substring("out".length) ?? "0"),
            targetHandle: parseInt(edge.targetHandle?.substring("in".length) ?? "0"),
            style: undefined,
        }));
    }

    getInternalEdges(): WasmEdge[] {
        return filterInternalEdges(this.getEdges(), this.getNodes());
    }

    getPlaced(isEditMode: boolean): [Node[], Edge[]] {
        if (isEditMode) {
            return [this.nodes, this.edges];
        } else {
            return removeBoundary(this.nodes, this.edges);
        }
    }

    getPlacedEdges(): PlacedWasmEdge[] {
        return this.edges;
    }

    getSelectedNodes() {
        return new Set(this.nodes.filter(node => node.selected).map(node => node.id));
    }

    applySetGraph({ nodes, edges }: WasmGraph, setUpdatedPortCounts: (updatedNodeIds: string[]) => void) {
        const placedNodes = nodes.map(node => ({
            ...node,
            position: this.positions[node.id] ?? { x: 0, y: 0 },
            selected: false,
        }));
        const placedEdges = edges.map(edge => ({
            ...edge,
            sourceHandle: `out${edge.sourceHandle}`,
            targetHandle: `in${edge.targetHandle}`,
            style: edgeStyle(edge.style)
        }));
        let newGraph = new RFGraph(placedNodes, placedEdges, this.positions);
        const updatedNodeIds = newGraph.updatePortCounts();
        setUpdatedPortCounts(updatedNodeIds);
        return newGraph;
    }

    applyRemoveEdge(edgeId: string) {
        return new RFGraph(this.nodes, this.edges.filter(edge => edge.id !== edgeId), this.positions);
    }

    applyNodesChange(changes: NodeChange[], setUpdatedPortCounts: (updatedNodeIds: string[]) => void) {
        if (changes.length === 0) {
            return this;
        }
        let newNodes = applyNodeChanges(changes, this.nodes) as PlacedWasmNode[];
        let newPositions = { ...this.positions };
        for (let node of newNodes) {
            newPositions[node.id] = node.position;
        }
        let newGraph = new RFGraph(newNodes, this.edges, newPositions);
        const updatedNodeIds = newGraph.updatePortCounts();
        setUpdatedPortCounts(updatedNodeIds);
        return newGraph;
    }

    applyEdgesChange(changes: EdgeChange[], setUpdatedPortCounts: (updatedNodeIds: string[]) => void) {
        let internalChanges = filterInternalEdgeChanges(changes, this.edges, this.nodes);
        const newEds = applyEdgeChanges(internalChanges, this.getPlacedEdges()) as PlacedWasmEdge[];
        let newGraph = new RFGraph(this.nodes, newEds, this.positions);
        const updatedNodeIds = newGraph.updatePortCounts();
        setUpdatedPortCounts(updatedNodeIds);
        return newGraph;
    }

    applyDeselectAll() {
        const newNodes = this.nodes.map(node => ({ ...node, selected: false }));
        return new RFGraph(newNodes, this.edges, this.positions);
    }

    applyAddNode(node: PlacedWasmNode, setUpdatedPortCounts: (updatedNodeIds: string[]) => void) {
        let newGraph = new RFGraph([...this.nodes, node], this.edges, this.positions);
        const updatedNodeIds = newGraph.updatePortCounts();
        setUpdatedPortCounts(updatedNodeIds);
        return newGraph;
    }

    updatePortCounts() {
        let updatedNodeIds: string[] = [];
        for (let node of this.nodes) {
            // Do not resize ports of boundary nodes
            if (node.type === "Boundary") {
                continue;
            }
            const outPorts = this.edges.filter(edge => edge.source === node.id).map(edge => edge.sourceHandle);
            const inPorts = this.edges.filter(edge => edge.target === node.id).map(edge => edge.targetHandle);
            const uniqueOutPorts = new Set(outPorts).size;
            const uniqueInPorts = new Set(inPorts).size;
            let updated = false;
            if (node.data.n_outputs === undefined || uniqueOutPorts >= node.data.n_outputs) {
                node.data.n_outputs = uniqueOutPorts + 1;
                updated = true;
            }
            if (node.data.n_inputs === undefined || uniqueInPorts >= node.data.n_inputs) {
                node.data.n_inputs = uniqueInPorts + 1;
                updated = true;
            }
            if (updated) {
                updatedNodeIds.push(node.id);
            }
        }
        return updatedNodeIds;
    }

    findEdgeToRemove() {
        // A map from boundary node ID to all edge ID that connect to it to internal nodes.
        const boundaryInternalEdges = new Map<string, string[]>();
        // A map from boundary node ID to all edge ID that connect to it to external nodes.
        const boundaryExternalEdges = new Map<string, string[]>();

        for (const edge of this.edges) {
            const [srcId, tgtId] = [edge.source, edge.target];
            const srcType = this.nodes.find((node) => node.id === srcId)?.type;
            const tgtType = this.nodes.find((node) => node.id === tgtId)?.type;
            if (!srcType || !tgtType || srcType !== "Boundary" && tgtType !== "Boundary") {
                continue;
            }
            if (srcType === "Internal" && tgtType === "Boundary") {
                boundaryInternalEdges.set(tgtId, [...(boundaryInternalEdges.get(tgtId) || []), edge.id]);
            } else if (srcType === "Boundary" && tgtType === "Internal") {
                boundaryInternalEdges.set(srcId, [...(boundaryInternalEdges.get(srcId) || []), edge.id]);
            } else if (srcType === "External" && tgtType === "Boundary") {
                boundaryExternalEdges.set(tgtId, [...(boundaryExternalEdges.get(tgtId) || []), edge.id]);
            } else if (srcType === "Boundary" && tgtType === "External") {
                boundaryExternalEdges.set(srcId, [...(boundaryExternalEdges.get(srcId) || []), edge.id]);
            } else if (srcType === "Boundary" && tgtType === "Boundary") {
                boundaryInternalEdges.set(srcId, [...(boundaryInternalEdges.get(srcId) || []), edge.id]);
                boundaryInternalEdges.set(tgtId, [...(boundaryInternalEdges.get(tgtId) || []), edge.id]);
            }
        }

        // Find a boundary edge that connects to a boundary with more than one edge..
        return [
            ...boundaryInternalEdges.values(), ...boundaryExternalEdges.values()
        ].map((edgeIds) => {
            return edgeIds.slice(1);
        }).find((edgeIds) => edgeIds.length > 0)?.[0];
    }

    isValidConnection(conn: Connection): boolean {
        const sourceNode = this.nodes.find((node) => node.id === conn.source);
        const targetNode = this.nodes.find((node) => node.id === conn.target);
        const isActive = (n: WasmNode) => n.type !== "External";
        return (
            typeof sourceNode !== "undefined" &&
            typeof targetNode !== "undefined" &&
            isActive(sourceNode) &&
            isActive(targetNode)
        );
    }
}

function useRFGraph({ nodes, edges }: WasmGraph, positions: Record<string, XYPosition>, setUpdatedPortCounts: (updatedNodeIds: string[]) => void) {
    const [graph, setGraphState] = useState<RFGraph>(RFGraph.initGraph({ nodes, edges }, positions));

    const setGraph = useCallback((newGraph: WasmGraph) => {
        console.log("setGraph:", newGraph);
        setGraphState(g => g.applySetGraph(newGraph, setUpdatedPortCounts));
    }, [setGraphState, setUpdatedPortCounts]);

    const onNodesChange = useCallback((changes: NodeChange[]) => {
        if (changes.length !== 0) {
            setGraphState(g => g.applyNodesChange(changes, setUpdatedPortCounts));
        }
    }, [setGraphState, setUpdatedPortCounts]);

    const onNodesChangeSelectOnly = useCallback((changes: NodeChange[]) => {
        const selectChanges = changes.filter(change => change.type === "select");
        if (selectChanges.length !== 0) {
            setGraphState(g => g.applyNodesChange(selectChanges, setUpdatedPortCounts));
        }
    }, [graph, setGraphState, setUpdatedPortCounts]);

    const deselectAll = useCallback(() => {
        setGraphState(g => g.applyDeselectAll());
    }, [setGraphState]);

    const onEdgesChange = useCallback((changes: EdgeChange[]) => {
        if (changes.length !== 0) {
            setGraphState(g => g.applyEdgesChange(changes, setUpdatedPortCounts));
        }
    }, [graph, setGraphState, setUpdatedPortCounts]);

    const appendNode = useCallback((node: PlacedWasmNode) => {
        setGraphState(g => g.applyAddNode(node, setUpdatedPortCounts));
    }, [setGraphState, setUpdatedPortCounts]);
    const { onDoubleClick, onInit } = useOnDoubleClick(appendNode);
    const onConnect = useCallback((conn: Connection) => {
        const edge = {
            id: uuid(),
            ...conn
        } as Edge;
        const edgeAdds = [{ item: edge, type: "add" as const }];
        onEdgesChange(edgeAdds);
    }, [onEdgesChange]);
    const isValidConnection = useCallback((conn: Connection) => graph.isValidConnection(conn), [graph]);

    // Enforce the boundary edge invariant.
    useEffect(() => {
        const edgeIdToRemove = graph.findEdgeToRemove();
        if (edgeIdToRemove) {
            setGraphState(g => g.applyRemoveEdge(edgeIdToRemove));
        }
    }, [graph, setGraphState]);

    return {
        graph,
        graphActions: {
            setGraph,
            deselectAll,
        },
        editHandlers: {
            onNodesChange,
            onEdgesChange,
            onConnect,
            onInit,
            isValidConnection,
            onDoubleClick,
        },
        viewHandlers: {
            onNodesChange: onNodesChangeSelectOnly,
            onEdgesChange: () => { },
            // onConnect: () => { },
            // onInit: () => { },
            // isValidConnection: () => true,
            // onDoubleClick: () => { },
        }
    };
}

export default useRFGraph;

function filterInternalEdges(edges: WasmEdge[], nodes: WasmNode[]): WasmEdge[] {
    const isInternal = (edge: WasmEdge) => {
        const source = nodes.find((node) => node.id === edge.source);
        const target = nodes.find((node) => node.id === edge.target);
        return source?.type !== "External" && target?.type !== "External";
    }
    return edges.filter(isInternal);
}

function useOnDoubleClick(appendNode: (node: PlacedWasmNode) => void) {
    const [reactFlowInstance, setReactFlowInstance] =
        useState<ReactFlowInstance | null>(null);

    const onDoubleClick = useCallback(
        (event: React.MouseEvent) => {
            if (reactFlowInstance) {
                const transform = reactFlowInstance.screenToFlowPosition({
                    x: event.clientX,
                    y: event.clientY,
                });

                const newNode = {
                    id: uuid(),
                    position: { x: transform.x, y: transform.y },
                    data: {
                        port_diff_id: "new",
                        n_inputs: 0,
                        n_outputs: 0,
                        label: "yolo",
                        color: undefined,
                    },
                    type: "Internal" as const,
                    selected: false,
                };
                appendNode(newNode);
            }
        },
        [reactFlowInstance, appendNode]
    );

    const onInit = useCallback(setReactFlowInstance, []);

    return { onDoubleClick, onInit };
}

function filterInternalEdgeChanges(
    changes: EdgeChange[],
    edges: PlacedWasmEdge[],
    nodes: PlacedWasmNode[],
) {
    return changes.filter((change) => {
        const { srcNode, tgtNode } = getEndsFromEdgeChange(
            change,
            edges,
            nodes,
        );
        const srcType = srcNode?.type;
        const tgtType = tgtNode?.type;
        if (!srcType || !tgtType) {
            // We don't understand the change, keep it conservatively
            console.log("Unknown edge change", change);
            return true;
        }
        if (srcType === "Internal" && tgtType === "Internal") {
            return true;
        } else if (srcType === "External" || tgtType === "External") {
            return false;
        } else {
            // Half boundary half internal
            return change.type !== "remove";
        }
    });
}

function getEndsFromEdgeChange(
    change: EdgeChange,
    edges: PlacedWasmEdge[],
    nodes: PlacedWasmNode[],
) {
    const edge =
        "id" in change
            ? edges.find((edge) => edge.id === change.id)
            : change.item;
    if (edge === undefined) {
        console.log("Edge for change not found", change);
        return { srcNode: undefined, tgtNode: undefined };
    }
    const srcNode = nodes.find((node) => node.id === edge.source);
    const tgtNode = nodes.find((node) => node.id === edge.target);
    return { srcNode, tgtNode };
}

export const dashedStyle: CSSProperties = {
    stroke: '#808080',
    strokeWidth: 1,
    strokeDasharray: '2,2'
}

function edgeStyle(style: string | undefined) {
    switch (style) {
        case "dashed":
            return dashedStyle;
        default: return {
            stroke: '#000',
            strokeWidth: 1,
        };
    }
}