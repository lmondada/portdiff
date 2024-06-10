import {
    Dispatch,
    SetStateAction,
    useCallback,
    useEffect,
    useState,
} from "react";
import ReactFlow, { Node, Edge, NodeChange, applyNodeChanges } from "reactflow";
import { getHierarchy, selectDiffs } from "../wasm_api";
import { placeGraph } from "../place_graph";
import { hierarchyNodeTypes } from "./port_diff_viewer/node_types";

type HierarchyViewerProps = {
    updateHierarchy: boolean;
    sendUpdatePortDiff: () => void;
};

function HierarchyViewer({
    updateHierarchy,
    sendUpdatePortDiff,
}: HierarchyViewerProps) {
    const [nodes, setNodes] = useState<Node[]>([]);
    const [edges, setEdges] = useState<Edge[]>([]);
    const [selectedNodes, setSelectedNodes] = useState<Set<string>>(new Set());

    useEffect(() => {
        const hierarchy = getHierarchy();
        const unplaced_nodes = Array.from(
            new Set(hierarchy.map(([parent, child]) => [parent, child]).flat()),
        ).map((id) => ({ id, data: null, type: "custom" }));
        const edges = hierarchy.map(([child, parent]) => ({
            id: `${parent}-${child}`,
            type: "step",
            source: parent,
            target: child,
        }));
        const positions = placeGraph({ nodes: unplaced_nodes, edges });
        const nodes = unplaced_nodes.map((node) => ({
            ...node,
            position: positions[node.id],
        }));
        setNodes(nodes);
        setEdges(edges);
    }, [updateHierarchy]);

    // Keep track of selected nodes
    useEffect(() => {
        const newNodes = nodes.filter((node) => node.selected);
        setSelectedNodes(new Set(newNodes.map((node) => node.id)));
        if (newNodes.length > 0) {
            selectDiffs(new Set(newNodes.map((node) => node.id)));
            sendUpdatePortDiff();
        }
    }, [nodes]);

    const view_handlers = graphHandlers(setNodes);

    return (
        <div style={{ height: "100%" }}>
            <ReactFlow
                nodes={nodes}
                edges={edges}
                nodeTypes={hierarchyNodeTypes}
                {...view_handlers}
            ></ReactFlow>
        </div>
    );
}

export default HierarchyViewer;

export function graphHandlers(setNodes: Dispatch<SetStateAction<Node[]>>) {
    const onNodesChange = useCallback(
        (changes: NodeChange[]) => {
            const allowedChanges = changes.filter(
                (change) =>
                    change.type === "select" || change.type === "position",
            );
            if (allowedChanges.length > 0) {
                setNodes((nds) => applyNodeChanges(allowedChanges, nds));
            }
        },
        [setNodes],
    );
    return { onNodesChange };
}
