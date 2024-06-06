import { XYPosition } from "reactflow";
import { useMemo } from "react";
import { Edge } from "reactflow";
import { PlacedWasmNode } from "./wasm_api";


export function removeBoundary(
    nodes: PlacedWasmNode[],
    edges: Edge[],
) {
    let boundaryEnds: { [key: string]: { source: string | undefined, target: string | undefined } } = {};
    for (const edge of edges) {
        const targetNode = nodes.find((node) => node.id === edge.target);
        const sourceNode = nodes.find((node) => node.id === edge.source);
        if (sourceNode && targetNode?.type === "Boundary") {
            if (!boundaryEnds[targetNode.id]) {
                boundaryEnds[targetNode.id] = { source: undefined, target: undefined };
            }
            boundaryEnds[targetNode.id].source = edge.source;
        }
        if (targetNode && sourceNode?.type === "Boundary") {
            if (!boundaryEnds[sourceNode.id]) {
                boundaryEnds[sourceNode.id] = { source: undefined, target: undefined };
            }
            boundaryEnds[sourceNode.id].target = edge.target;
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

    const boundaryEdges = Object.values(boundaryEnds).map(({ source, target }) => {
        if (!source || !target) {
            return null;
        }
        return { id: `${source}-${target}`, source, target };
    }).filter((edge) => edge !== null) as Edge[];

    const nodesNoBoundary = nodes.filter((node) => node.type !== "Boundary");
    const edgesNoBoundary = [...boundaryEdges, ...nonBoundaryEdges];

    return { nodesNoBoundary, edgesNoBoundary };
}
