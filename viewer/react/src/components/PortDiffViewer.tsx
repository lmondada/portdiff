import ReactFlow, {
    Background,
    BackgroundVariant,
    Controls,
    Panel,
    SelectionMode,
} from "reactflow";

import EditModeButton from "./port_diff_viewer/EditModeButton";
import { useEffect } from "react";
import { nodeTypes } from "./port_diff_viewer/node_types";
import usePortDiffState from "../hooks/usePortDiffState";
import CommitButton from "./port_diff_viewer/CommitButton";
import UpdatePorts from "./port_diff_viewer/UpdatePorts";

type PortDiffViewerProps = {
    updatePortDiff: boolean;
    sendUpdateHierarchy: () => void;
};
function PortDiffViewer({
    updatePortDiff,
    sendUpdateHierarchy,
}: PortDiffViewerProps) {
    const {
        isEditMode,
        toggleEditMode,
        isCommitted,
        commitSelection,
        editHandlers,
        viewHandlers,
        nodes,
        edges,
        drainUpdatePortCounts,
    } = usePortDiffState(updatePortDiff, sendUpdateHierarchy);

    // Pressing E toggles edit mode
    useEffect(() => {
        const handleKeyDown = (event: KeyboardEvent) => {
            if (event.key === "e" || event.key === "E") {
                toggleEditMode();
            }
        };

        window.addEventListener("keydown", handleKeyDown);
        return () => {
            window.removeEventListener("keydown", handleKeyDown);
        };
    }, []);

    const bg_opts = pick_background(isEditMode);
    const flow_opts = pick_flow_options(isEditMode);

    return (
        <div style={{ height: "100%" }}>
            <ReactFlow
                nodes={nodes}
                edges={edges}
                nodeTypes={nodeTypes}
                {...(isEditMode ? editHandlers : viewHandlers)}
                {...flow_opts}
            >
                <Background {...bg_opts} />
                <Controls />
                <Panel position="top-right">
                    {!isCommitted ? (
                        <CommitButton onClick={commitSelection} />
                    ) : null}
                    <EditModeButton
                        isEditMode={isEditMode}
                        toggleEditMode={toggleEditMode}
                    />
                </Panel>
                <UpdatePorts drainUpdatePortCounts={drainUpdatePortCounts} />
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
