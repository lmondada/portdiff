import { Dispatch, SetStateAction, useCallback, useEffect, useMemo, useState } from "react";
import { initApp, rewriteGraph, selectNodes, currentGraph } from "../wasm_api";
import { NodeChange, NodeSelectionChange } from "reactflow";
import useRFGraph from "../RFGraph";
import placeGraph from "../place_graph";

/** A communication queue to update port counts */
function useUpdatePortCounts() {
    const [updatedPortCounts, setUpdatedPortCounts] = useState<string[]>([]);
    return {
        pushUpdatePortCounts: useCallback((nodeIds: string | string[]) => {
            nodeIds = typeof nodeIds === "string" ? [nodeIds] : nodeIds;
            if (nodeIds.length > 0) {
                setUpdatedPortCounts(prev => [...prev, ...nodeIds]);
            }
        }, [setUpdatedPortCounts]),
        drainUpdatePortCounts: useCallback(() => {
            const nodeIds = updatedPortCounts;
            if (nodeIds.length > 0) {
                setUpdatedPortCounts(prev => prev.filter(id => !nodeIds.includes(id)));
            }
            return nodeIds;
        }, [updatedPortCounts, setUpdatedPortCounts]),
    }
}

function usePortDiffState(updatePortDiff: boolean, sendUpdateHierarchy: () => void) {
    const [isEditMode, setIsEditMode] = useState(false);
    const [isCommitted, setIsCommitted] = useState(true);
    const { pushUpdatePortCounts, drainUpdatePortCounts } = useUpdatePortCounts();

    const initGraph = useMemo(() => initApp(), []);
    const initPositions = useMemo(() => placeGraph(initGraph), [initGraph]);
    const { graph, graphActions, editHandlers, viewHandlers } = useRFGraph(initGraph, initPositions, pushUpdatePortCounts);

    // Update whether state is committed
    useEffect(() => {
        if (isEditMode) {
            setIsCommitted(true);
        } else {
            setIsCommitted(graph.getSelectedNodes().size === 0);
        }
    }, [graph, isEditMode])


    // Update port diff when required
    useEffect(() => {
        const g = currentGraph();
        graphActions.setGraph(g);
    }, [updatePortDiff, graphActions.setGraph]);

    const toggleEditMode = useCallback(() => setIsEditMode((mode) => {
        if (mode) {
            // We are leaving edit mode, need to commit changes
            const internalEdges = graph.getInternalEdges();
            const g = rewriteGraph(internalEdges);
            graphActions.setGraph(g);
            setIsCommitted(true);
            sendUpdateHierarchy();
        }
        return !mode;
    }), [graph, sendUpdateHierarchy]);

    const commitSelection = useCallback(() => {
        const g = selectNodes(graph.getSelectedNodes());
        graphActions.setGraph(g);
        setIsCommitted(true);
        graphActions.deselectAll();
        sendUpdateHierarchy();
    }, [graph, graphActions.deselectAll, graphActions.setGraph, sendUpdateHierarchy]);

    let [nodes, edges] = useMemo(() => graph.getPlaced(isEditMode), [graph, isEditMode]);
    return {
        nodes, edges,
        isEditMode, toggleEditMode,
        isCommitted,
        commitSelection,
        editHandlers,
        viewHandlers,
        drainUpdatePortCounts,
    };
}

export default usePortDiffState;

