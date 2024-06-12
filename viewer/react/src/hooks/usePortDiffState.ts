import { Dispatch, SetStateAction, useCallback, useEffect, useMemo, useState } from "react";
import { initApp, rewriteGraph, selectNodes, currentGraph, WasmEdge, expandBoundary, WasmGraph } from "../wasm_api";
import { Edge, NodeChange, NodeSelectionChange } from "reactflow";
import useRFGraph from "../RFGraph";
import placeGraph from "../place_graph";
import RFGraph from "../RFGraph";

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

    const onEdgeDoubleClick = useCallback((event: React.MouseEvent<MouseEvent>, edge: Edge) => {
        console.log("onEdgeDoubleClick: ", edge);
        try {
            expandBoundary(edge.id);
        } catch (e) {
            console.error(e);
            return;
        }
        sendUpdateHierarchy();
    }, [sendUpdateHierarchy]);
    (viewHandlers as any).onEdgeDoubleClick = onEdgeDoubleClick;

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

    const [prevInternalEdges, setPrevInternalEdges] = useState<WasmEdge[]>([]);

    const toggleEditMode = useCallback(() => setIsEditMode((mode) => {
        const internalEdges = graph.getInternalEdges();
        if (mode && (internalEdges.length !== prevInternalEdges.length || internalEdges.some((edge, index) => edge.id != prevInternalEdges[index].id))) {
            // We are leaving edit mode, need to commit changes
            const g = rewriteGraph(internalEdges);
            graphActions.setGraph(g);
            setIsCommitted(true);
            sendUpdateHierarchy();
        } else if (!mode) {
            // Record the internal edges when entering edit mode for comparison
            setPrevInternalEdges(internalEdges);
        }
        return !mode;
    }), [graph, sendUpdateHierarchy, prevInternalEdges, setPrevInternalEdges]);

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

