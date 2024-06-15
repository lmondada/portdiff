import { dashedStyle } from "./RFGraph";
import { WasmEdge, WasmNode } from "./wasm_api";


export function removeBoundary<
    N extends { id: string, type: string }, P
>(
    nodes: N[],
    edges: { id: string, source: string, target: string, sourceHandle: P, targetHandle: P }[],
): [N[], { id: string, source: string, target: string, sourceHandle?: P, targetHandle?: P }[]] {
    const boundaryEdges = [];
    for (const edge of edges) {
        const fstEdge = edge;
        let lastEdge = edge as typeof edge | undefined;
        const sourceNode = nodes.find((node) => node.id === edge.source);
        let targetNode = nodes.find((node) => node.id === lastEdge?.target);
        if (!sourceNode || !targetNode || targetNode.type !== "Boundary" || sourceNode.type === "Boundary") {
            continue
        }
        const boundaryId = targetNode.id;
        while (targetNode?.type === "Boundary") {
            lastEdge = edges.find(e => e.source === targetNode?.id);
            targetNode = nodes.find((node) => node.id === lastEdge?.target);
        }
        if (typeof lastEdge !== "undefined") {
            boundaryEdges.push({
                id: `${boundaryId}-1`,
                source: fstEdge.source,
                sourceHandle: fstEdge.sourceHandle,
                target: lastEdge.target,
                targetHandle: lastEdge.targetHandle,
                style: dashedStyle,
            });
        }
    }

    const nonBoundaryEdges = edges.filter((edge) => {
        const targetNode = nodes.find((node) => node.id === edge.target);
        const sourceNode = nodes.find((node) => node.id === edge.source);
        if (!sourceNode || !targetNode) {
            return false;
        }
        return targetNode.type !== "Boundary" && sourceNode.type !== "Boundary";
    });

    // use the boundary ID for the edge ID so we can recover it later
    // const boundaryEdges = Object.entries(boundaryEnds).map(([boundaryId, { source, sourceHandle, target, targetHandle }]) => {
    //     if (!source || !target) {
    //         return null;
    //     }
    //     return { id: boundaryId, source, sourceHandle, target, targetHandle, style: dashedStyle };
    // }).filter((edge) => edge !== null) as { id: string, source: string, sourceHandle: P, target: string, targetHandle: P }[];

    const nodesNoBoundary = nodes.filter((node) => node.type !== "Boundary");
    const edgesNoBoundary = [...boundaryEdges, ...nonBoundaryEdges];

    return [nodesNoBoundary, edgesNoBoundary];
}
