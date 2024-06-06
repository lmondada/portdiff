import { Dispatch, SetStateAction, useCallback, useEffect, useMemo, useState } from "react";
import { initApp, WasmNode, rewriteGraph, PlacedWasmNode, PlacedWasmEdge, WasmEdge, WasmGraph } from "../wasm_api";
import { removeBoundary } from "../boundary";
import useEditModeHandlers from "./useEditModeHandlers";
import { Edge } from "reactflow";
import { place_edges, set_port_numbers } from "../place_graph";

function updateGraphCallback(
    setNodes: Dispatch<SetStateAction<PlacedWasmNode[]>>,
    setEdges: Dispatch<SetStateAction<PlacedWasmEdge[]>>,
) {
    return (newGraph: WasmGraph) => {
        console.log(newGraph);
        set_port_numbers(newGraph);
        let placed_edges = place_edges(newGraph.edges);
        // Inherit placement of existing nodes
        setNodes((prevNodes) => {
            const prevNodesMap = new Map(prevNodes.map(node => [node.id, node]));
            return newGraph.nodes.map(newNode => {
                const prevNode = prevNodesMap.get(newNode.id) ?? { position: { x: 0, y: 0 } };
                return { ...prevNode, ...newNode };
            });
        });
        setEdges(placed_edges);
    };
}

function usePortDiffState() {
    const [isEditMode, setIsEditMode] = useState(false);
    const initGraph = useMemo(() => {
        return initApp();
    }, []);
    const [nodes, setNodes] = useState<PlacedWasmNode[]>(initGraph.nodes);
    const [edges, setEdges] = useState<PlacedWasmEdge[]>(initGraph.edges);

    const updateGraph = useCallback(updateGraphCallback(setNodes, setEdges), [setNodes, setEdges]);

    const { nodesNoBoundary, edgesNoBoundary } = useMemo(() => {
        return removeBoundary(nodes, edges);
    }, [nodes, edges]);

    // Event handlers for modifying the graph in edit mode
    const edit_handlers = useEditModeHandlers({
        nodes,
        edges,
        setNodes,
        setEdges,
    });
    const view_handlers = {};

    const toggleEditMode = useCallback(() => setIsEditMode((mode) => {
        if (mode) {
            // We are leaving edit mode, need to commit changes
            const internalEdges = filterInternalEdges(edges, nodes);
            const g = rewriteGraph(internalEdges);
            updateGraph(g);
        }
        return !mode;
    }), [nodes, edges]);

    return {
        nodes, edges, setNodes, setEdges,
        nodesNoBoundary, edgesNoBoundary,
        isEditMode,
        toggleEditMode,
        edit_handlers,
        view_handlers,
    }
}

function filterInternalEdges(edges: Edge[], nodes: WasmNode[]): WasmEdge[] {
    const isInternal = (edge: Edge) => {
        const source = nodes.find((node) => node.id === edge.source);
        const target = nodes.find((node) => node.id === edge.target);
        return source?.type !== "External" && target?.type !== "External";
    }
    return edges.filter(isInternal).map((edge) => {
        return {
            id: edge.id,
            source: edge.source,
            target: edge.target,
            sourceHandle: parseInt(edge.sourceHandle?.slice(3) ?? "0"),
            targetHandle: parseInt(edge.targetHandle?.slice(2) ?? "0"),
        } as WasmEdge;
    });
}

export default usePortDiffState;

