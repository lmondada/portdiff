import { Dispatch, SetStateAction, useCallback, useEffect, useMemo, useState } from "react";
import { initApp, WasmNode, rewriteGraph, PlacedWasmNode, PlacedWasmEdge, WasmEdge, WasmGraph, selectNodes } from "../wasm_api";
import { removeBoundary } from "../boundary";
import useEditModeHandlers from "./useEditModeHandlers";
import { Edge, NodeChange, useUpdateNodeInternals } from "reactflow";
import { place_edges, set_port_numbers } from "../place_graph";

function updateGraphCallback(
    setNodes: Dispatch<SetStateAction<PlacedWasmNode[]>>,
    setEdges: Dispatch<SetStateAction<PlacedWasmEdge[]>>,
) {
    return (newGraph: WasmGraph) => {
        set_port_numbers(newGraph);
        let placed_edges = place_edges(newGraph.edges);
        // Inherit placement of existing nodes (but deselect)
        setNodes((prevNodes) => {
            const prevNodesMap = new Map(prevNodes.map(node => [node.id, node]));
            return newGraph.nodes.map(newNode => {
                const prevNode = prevNodesMap.get(newNode.id) ?? { position: { x: 0, y: 0 } };
                return { ...prevNode, ...newNode, selected: false };
            });
        });
        setEdges(placed_edges);
    };
}

function usePortDiffState() {
    const initGraph = useMemo(() => {
        return initApp();
    }, []);
    const [nodes, setNodes] = useState<PlacedWasmNode[]>(initGraph.nodes);
    const [edges, setEdges] = useState<PlacedWasmEdge[]>(initGraph.edges);
    const [isEditMode, setIsEditMode] = useState(false);
    const [isCommitted, setIsCommitted] = useState(true);
    const [selectedNodes, setSelectedNodes] = useState<Set<string>>(new Set());
    const [updatedPortCounts, setUpdatedPortCounts] = useState<string[]>([]);

    const updateGraph = useCallback(updateGraphCallback(setNodes, setEdges), [setNodes, setEdges]);

    // Keep track of selected nodes
    useEffect(() => {
        const newNodes = nodes.filter(node => node.type === "Internal" && node.selected);
        setSelectedNodes(new Set(newNodes.map(node => node.id)));
    }, [nodes]);

    // Update whether state is committed
    useEffect(() => {
        if (isEditMode) {
            setIsCommitted(true);
        } else {
            setIsCommitted(selectedNodes.size === 0);
        }
    }, [selectedNodes, isEditMode])

    // Adjust the number of ports in the graph
    useEffect(() => {
        const updatedNodeIds = updatePortCounts(nodes, edges, setNodes);
        if (updatedNodeIds.length > 0) {
            setUpdatedPortCounts(updatedNodeIds);
        }
    }, [nodes, edges, setNodes]);

    // Remove boundary nodes in non-edit mode
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
    const view_handlers = useMemo(() => viewHandlers(setNodes), [setNodes]);

    const toggleEditMode = useCallback(() => setIsEditMode((mode) => {
        if (mode) {
            // We are leaving edit mode, need to commit changes
            const internalEdges = filterInternalEdges(edges, nodes);
            const g = rewriteGraph(internalEdges);
            updateGraph(g);
            setIsCommitted(true);
        }
        return !mode;
    }), [nodes, edges]);

    const commitSelection = useCallback(() => {
        const g = selectNodes(selectedNodes);
        updateGraph(g);
        setIsCommitted(true);
        setSelectedNodes(new Set());
    }, [selectedNodes]);

    const resetUpdatedPortCounts = useCallback(() => {
        setUpdatedPortCounts([]);
    }, []);

    return {
        nodes, edges,
        nodesNoBoundary, edgesNoBoundary,
        isEditMode, toggleEditMode,
        isCommitted,
        commitSelection,
        edit_handlers,
        view_handlers,
        updatedPortCounts,
        resetUpdatedPortCounts,
    };
}

function updatePortCounts(nodes: WasmNode[], edges: PlacedWasmEdge[], setNodes: Dispatch<SetStateAction<PlacedWasmNode[]>>) {
    function setPortCount(nodeId: string, count: number, field: "n_outputs" | "n_inputs") {
        setNodes(nodes => nodes.map(node => {
            if (node.type === "Internal" && node.id === nodeId) {
                return { ...node, data: { ...node.data, [field]: count } }
            } else if (node.type === "External" && node.id === nodeId) {
                return { ...node, data: { ...node.data, [field]: count } }
            }
            return node;
        }));
    }
    const setOutputPortCount = (nodeId: string, count: number) => setPortCount(nodeId, count, "n_outputs");
    const setInputPortCount = (nodeId: string, count: number) => setPortCount(nodeId, count, "n_inputs");

    let updatedNodeIds: string[] = [];
    for (const node of nodes) {
        // Do not resize ports of boundary nodes
        if (node.type === "Boundary") {
            continue;
        }
        const outPorts = edges.filter(edge => edge.source === node.id).map(edge => edge.sourceHandle);
        const inPorts = edges.filter(edge => edge.target === node.id).map(edge => edge.targetHandle);
        const uniqueOutPorts = new Set(outPorts).size;
        const uniqueInPorts = new Set(inPorts).size;
        let updated = false;
        if (uniqueOutPorts >= node.data.n_outputs) {
            setOutputPortCount(node.id, uniqueOutPorts + 1);
            updated = true;
        }
        if (uniqueInPorts >= node.data.n_inputs) {
            setInputPortCount(node.id, uniqueInPorts + 1);
            updated = true;
        }
        if (updated) {
            updatedNodeIds.push(node.id);
        }
    }
    return updatedNodeIds;
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

function viewHandlers(setNodes: Dispatch<SetStateAction<PlacedWasmNode[]>>) {
    return {
        onNodesChange: (changes: NodeChange[]) => {
            for (const change of changes) {
                if (change.type === "select") {
                    setNodes((nodes) => {
                        return nodes.map((node) => {
                            if (node.id === change.id) {
                                return { ...node, selected: change.selected };
                            }
                            return node;
                        });
                    });
                }
            }
        }
    };
}

export default usePortDiffState;

