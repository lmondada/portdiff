import { Edge } from "reactflow";
import { PlacedWasmEdge, WasmNode } from "../wasm_api";
import { Dispatch, SetStateAction, useEffect, useMemo } from "react";

/**
 * Enforce the boundary edge invariant.
 *
 * Every boundary vertex has exactly two edges connecting it to one internal and
 * one external vertex.
 *
 * If a new edge is added to a boundary vertex, remove the edge that is already
 * connected to the internal/external vertex.
 */
function useRemoveDoubleBoundaryEdges(
    edges: Edge[],
    nodes: WasmNode[],
    setEdges: Dispatch<SetStateAction<PlacedWasmEdge[]>>,
) {
    useEffect(() => {
        const edgeIdToRemove = findEdgeToRemove(edges, nodes);
        if (edgeIdToRemove) {
            setEdges((edges) => edges.filter((edge) => edge.id !== edgeIdToRemove));
        }
    }, [edges, nodes, setEdges]);
}

function findEdgeToRemove(edges: Edge[], nodes: WasmNode[]) {
    // A map from boundary node ID to all edge ID that connect to it to internal nodes.
    const boundaryInternalEdges = new Map<string, string[]>();
    // A map from boundary node ID to all edge ID that connect to it to external nodes.
    const boundaryExternalEdges = new Map<string, string[]>();

    for (const edge of edges) {
        const [srcId, tgtId] = [edge.source, edge.target];
        const srcType = nodes.find((node) => node.id === srcId)?.type;
        const tgtType = nodes.find((node) => node.id === tgtId)?.type;
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

export default useRemoveDoubleBoundaryEdges;

