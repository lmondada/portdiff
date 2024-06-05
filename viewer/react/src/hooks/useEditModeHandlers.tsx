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
import { Node, node_type_name } from "../wasm_api";

// function initBoundary(nodes: Node[], edges: Edge[]) {
//     const edgesOnBoundary = edges.filter((edge) => isBoundaryEdge(edge, nodes));
//     const initBoundaryNodes: BoundaryNode[] = edgesOnBoundary.map((edge) =>
//         fromBoundaryEdge(edge, nodes),
//     );
//     const initBoundaryEdges = edgesOnBoundary
//         .map((e, i) => {
//             return [
//                 {
//                     id: `${e.id}-1`,
//                     source: e.source,
//                     target: initBoundaryNodes[i].id,
//                 },
//                 {
//                     id: `${e.id}-2`,
//                     source: initBoundaryNodes[i].id,
//                     target: e.target,
//                 },
//             ];
//         })
//         .flat();
//     return { initBoundaryNodes, initBoundaryEdges };
// }

// function useAddBoundary(nodes: Node[], edges: Edge[]) {
//     const { initBoundaryNodes, initBoundaryEdges } = useMemo(
//         () => initBoundary(nodes, edges),
//         [nodes, edges],
//     );
//     const [boundaryEdges, setBoundaryEdges] = useState(initBoundaryEdges);
//     const [boundaryNodes, setBoundaryNodes] = useState(initBoundaryNodes);

//     const nonBoundaryEdges = edges.filter(
//         (edge) => !isBoundaryEdge(edge, nodes),
//     );

//     const editNodes = [...nodes, ...boundaryNodes];
//     const editEdges = [...nonBoundaryEdges, ...boundaryEdges];

//     return {
//         boundaryNodes,
//         boundaryEdges,
//         editNodes,
//         editEdges,
//         setBoundaryNodes,
//         setBoundaryEdges,
//     };
// }

function useEditModeHandlers({
    nodes,
    edges,
    setNodes,
    setEdges,
}: {
    nodes: Node[];
    edges: Edge[];
    setNodes: Dispatch<SetStateAction<Node[]>>;
    setEdges: Dispatch<SetStateAction<Edge[]>>;
}) {
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
                    id: `${nodes.length + 1}`,
                    position: { x: transform.x, y: transform.y },
                    data: {
                        port_diff_id: "ad",
                        n_inputs: 0,
                        n_outputs: 0,
                        label: "yolo",
                    },
                    type: "Internal" as const,
                };
                setNodes((prevNodes) => [...prevNodes, newNode]);
            }
        },
        [reactFlowInstance, nodes],
    );

    const onInit = useCallback(setReactFlowInstance, []);
    const onNodesChange = useCallback((changes: NodeChange[]) => {
        setNodes((nds) => {
            return applyNodeChanges(changes, nds) as Node[];
        });
    }, []);
    const onEdgesChange = useCallback((changes: EdgeChange[]) => {
        setEdges((eds) => applyEdgeChanges(changes, eds));
    }, []);
    const onConnect = useCallback((params: Connection) => {
        setEdges((eds) => addEdge(params, eds));
    }, []);
    const isValidConnection = useCallback(
        (params: Connection) => {
            const sourceNode = nodes.find((node) => node.id === params.source);
            const targetNode = nodes.find((node) => node.id === params.target);
            const isActive = (n: Node) => n.type !== "External";
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

export default useEditModeHandlers;
