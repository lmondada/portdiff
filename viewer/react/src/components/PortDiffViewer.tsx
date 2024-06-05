import ReactFlow, {
    Background,
    BackgroundVariant,
    Connection,
    Controls,
    Edge,
    EdgeChange,
    NodeChange,
    NodeTypes,
    Panel,
    ReactFlowInstance,
    SelectionMode,
    addEdge,
    applyEdgeChanges,
    applyNodeChanges,
} from "reactflow";

import EditModeButton from "./port_diff_viewer/EditModeButton";
import { useEffect, useMemo, useState } from "react";
import { nodeTypes } from "./port_diff_viewer/node_types";
import UpdatePorts from "./port_diff_viewer/UpdatePorts";
import { createGraph } from "../wasm_api";
import useEditModeHandlers from "../hooks/useEditModeHandlers";
import { removeBoundary } from "../boundary";

function PortDiffViewer() {
    const [isEditMode, setIsEditMode] = useState(false);
    let initGraph = createGraph();
    const [nodes, setNodes] = useState(initGraph.nodes);
    const [edges, setEdges] = useState(initGraph.edges);

    const { nodesNoBoundary, edgesNoBoundary } = useMemo(() => {
        return removeBoundary(nodes, edges);
    }, [nodes, edges]);

    // Pressing E toggles edit mode
    useEffect(() => {
        const handleKeyDown = (event: KeyboardEvent) => {
            if (event.key === "e" || event.key === "E") {
                setIsEditMode((prevMode) => !prevMode);
            }
        };

        window.addEventListener("keydown", handleKeyDown);
        return () => {
            window.removeEventListener("keydown", handleKeyDown);
        };
    }, []);

    // Event handlers for modifying the graph in edit mode
    const edit_handlers = useEditModeHandlers({
        nodes,
        edges,
        setNodes,
        setEdges,
    });
    const view_handlers = {};

    const bg_opts = pick_background(isEditMode);
    const flow_opts = pick_flow_options(isEditMode);

    return (
        <div style={{ height: "100%" }}>
            <ReactFlow
                nodes={isEditMode ? nodes : nodesNoBoundary}
                edges={isEditMode ? edges : edgesNoBoundary}
                nodeTypes={nodeTypes}
                {...(isEditMode ? edit_handlers : view_handlers)}
                {...flow_opts}
            >
                <Background {...bg_opts} />
                <Controls />
                <Panel position="top-right">
                    <EditModeButton
                        isEditMode={isEditMode}
                        toggleEditMode={() => setIsEditMode(!isEditMode)}
                    />
                </Panel>
                <UpdatePorts nodes={nodes} edges={edges} setNodes={setNodes} />
            </ReactFlow>
        </div>
    );
}

function pick_background(isEditMode: boolean) {
    const variant = isEditMode
        ? ("lines" as BackgroundVariant)
        : ("dots" as BackgroundVariant);
    const color = isEditMode ? "#FFCCCC" : "#333";
    return { variant, color };
}

export function pick_flow_options(isEditMode: boolean) {
    const nodesDraggable = isEditMode;
    const nodesConnectable = isEditMode;
    const selectionOnDrag = false;
    const panOnDrag = true;
    const zoomOnDoubleClick = false;
    const selectionMode = "partial" as SelectionMode;
    return {
        zoomOnDoubleClick,
        nodesDraggable,
        nodesConnectable,
        selectionOnDrag,
        panOnDrag,
        selectionMode,
    };
}

export default PortDiffViewer;
