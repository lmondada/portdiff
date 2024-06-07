import {
    Dispatch,
    SetStateAction,
    useCallback,
    useMemo,
    useState,
} from "react";
import {
    Connection,
    Edge,
    EdgeChange,
    NodeChange,
    ReactFlowInstance,
    addEdge,
    applyEdgeChanges,
    applyNodeChanges,
} from "reactflow";
import { InternalNode, PlacedWasmEdge, PlacedWasmNode } from "../wasm_api";
import useRemoveDoubleBoundaryEdges from "./useRemoveDoubleBoundaryEdges";
import { v4 as uuid } from "uuid";

function useEditModeHandlers({
    nodes,
    edges,
    setNodes,
    setEdges,
}: {
    nodes: PlacedWasmNode[];
    edges: Edge[];
    setNodes: Dispatch<SetStateAction<PlacedWasmNode[]>>;
    setEdges: Dispatch<SetStateAction<PlacedWasmEdge[]>>;
}) {
    useRemoveDoubleBoundaryEdges(edges, nodes, setEdges);

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
                setNodes((prevNodes) => [...prevNodes, newNode]);
            }
        },
        [reactFlowInstance]
    );

    const onInit = useCallback(setReactFlowInstance, []);
    const onNodesChange = useCallback((changes: NodeChange[]) => {
        setNodes((nds) => {
            const internalChanges = filterInternalNodeChanges(changes, nds);
            return applyNodeChanges(internalChanges, nds) as PlacedWasmNode[];
        });
    }, []);
    const onEdgesChange = useCallback((changes: EdgeChange[]) => {
        setEdges((eds) => {
            let internalChanges = filterInternalEdgeChanges(changes, eds, nodes);
            return applyEdgeChanges(internalChanges, eds) as PlacedWasmEdge[]
        });
    }, [nodes]);
    const onConnect = useCallback((conn: Connection) => {
        const edge = {
            id: uuid(),
            ...conn
        } as Edge;
        const edgeAdds = [{ item: edge, type: "add" as const }];
        onEdgesChange(edgeAdds);
    }, [onEdgesChange]);
    const isValidConnection = useCallback(
        (params: Connection) => {
            const sourceNode = nodes.find((node) => node.id === params.source);
            const targetNode = nodes.find((node) => node.id === params.target);
            const isActive = (n: PlacedWasmNode) => n.type !== "External";
            return (
                sourceNode &&
                targetNode &&
                isActive(sourceNode) &&
                isActive(targetNode)
            );
        },
        [nodes],
    );

    return {
        onDoubleClick,
        onNodesChange,
        onEdgesChange,
        onConnect,
        onInit,
        isValidConnection,
    };
}

function filterInternalNodeChanges(changes: NodeChange[], nodes: PlacedWasmNode[]) {
    return changes.filter((change) => {
        const node =
            "id" in change
                ? nodes.find((node) => node.id === change.id)
                : change.item;
        switch (node?.type) {
            case "Internal":
                return true;
            case "External":
                return false;
            case "Boundary":
                return change.type !== "remove";
            default:
                return true;
        }
    });
}

function filterInternalEdgeChanges(
    changes: EdgeChange[],
    edges: Edge[],
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
    edges: Edge[],
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
export default useEditModeHandlers;
