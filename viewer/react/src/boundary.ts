import { XYPosition } from "reactflow";
import { useMemo } from "react";
import { Edge } from "reactflow";
import { PlacedWasmNode } from "./wasm_api";


export function removeBoundary(
    nodes: PlacedWasmNode[],
    edges: Edge[],
) {
    let boundaryEnds: {
        [key: string]: {
            source: string | undefined,
            sourceHandle: string | undefined,
            target: string | undefined,
            targetHandle: string | undefined,
        }
    } = {};
    for (const edge of edges) {
        const targetNode = nodes.find((node) => node.id === edge.target);
        const sourceNode = nodes.find((node) => node.id === edge.source);
        if (sourceNode && targetNode?.type === "Boundary") {
            if (!boundaryEnds[targetNode.id]) {
                boundaryEnds[targetNode.id] = { source: undefined, sourceHandle: undefined, target: undefined, targetHandle: undefined };
            }
            boundaryEnds[targetNode.id].source = edge.source;
            boundaryEnds[targetNode.id].sourceHandle = edge.sourceHandle ?? undefined;
        }
        if (targetNode && sourceNode?.type === "Boundary") {
            if (!boundaryEnds[sourceNode.id]) {
                boundaryEnds[sourceNode.id] = { source: undefined, sourceHandle: undefined, target: undefined, targetHandle: undefined };
            }
            boundaryEnds[sourceNode.id].target = edge.target;
            boundaryEnds[sourceNode.id].targetHandle = edge.targetHandle ?? undefined;
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

    const boundaryEdges = Object.values(boundaryEnds).map(({ source, sourceHandle, target, targetHandle }) => {
        if (!source || !target) {
            return null;
        }
        return { id: `${source}-${target}`, source, sourceHandle, target, targetHandle };
    }).filter((edge) => edge !== null) as Edge[];

    const nodesNoBoundary = nodes.filter((node) => node.type !== "Boundary");
    const edgesNoBoundary = [...boundaryEdges, ...nonBoundaryEdges];

    return { nodesNoBoundary, edgesNoBoundary };
}
