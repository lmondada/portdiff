import { XYPosition } from "reactflow";
import { PlacedWasmEdge, PlacedWasmGraph, PlacedWasmNode, WasmEdge, WasmGraph, WasmNode } from "./wasm_api";
import Dagre from "@dagrejs/dagre";

export function placeGraph<
    N extends { id: string },
    E extends { source: string, target: string }
>(g: { nodes: N[], edges: E[] }): Record<string, XYPosition> {
    // Copied straight from https://reactflow.dev/learn/layouting/layouting
    const dagre = new Dagre.graphlib.Graph().setDefaultEdgeLabel(() => ({}));

    const getLayoutedElements = (nodes: N[], edges: E[]) => {
        dagre.setGraph({ rankdir: "TB" });

        edges.forEach((edge) => dagre.setEdge(edge.source, edge.target));
        nodes.forEach((node) => dagre.setNode(node.id, node));

        Dagre.layout(dagre);

        return {
            nodes: nodes.map((node) => {
                const position = dagre.node(node.id);
                // TODO: We are shifting the dagre node position (anchor=center center) to the top left
                // so it matches the React Flow node anchor point (top left).
                const x = position.x * 3;
                const y = position.y * 3;

                return { ...node, position: { x, y }, selected: false };
            }),
            edges,
        };
    };

    const layouted = getLayoutedElements(g.nodes, g.edges);

    return layouted.nodes.reduce((acc, node) => {
        acc[node.id] = node.position;
        return acc;
    }, {} as Record<string, XYPosition>);
}

export function placedNodes(nodes: WasmNode[], positions: Record<string, XYPosition>, selectedNodes: Set<string>): PlacedWasmNode[] {
    console.log(selectedNodes);
    return nodes.map((node) => ({
        ...node,
        position: positions[node.id],
        selected: selectedNodes.has(node.id),
    }));
}

export function unplaceNode(node: PlacedWasmNode): WasmNode {
    return {
        type: node.type,
        id: node.id,
        data: node.data,
    } as WasmNode;
}

// export function placedEdges(edges: WasmEdge[]): PlacedWasmEdge[] {
//     return edges.map((edge) => ({
//         ...edge,
//         sourceHandle: `out${edge.sourceHandle}`,
//         targetHandle: `in${edge.targetHandle}`,
//     }));
// }

// export function unplaceEdge(edge: PlacedWasmEdge): WasmEdge {
//     return {
//         ...edge,
//         sourceHandle: parseInt(edge.sourceHandle.substring("out".length)),
//         targetHandle: parseInt(edge.targetHandle.substring("in".length)),
//     };
// }

export function set_port_numbers(g: WasmGraph) {
    // Set the number of ports
    const maxInputsMap = new Map<string, number>();
    const maxOutputsMap = new Map<string, number>();
    for (const edge of g.edges) {
        const maxInput = maxInputsMap.get(edge.source) || 0;
        const maxOutput = maxOutputsMap.get(edge.target) || 0;
        maxInputsMap.set(edge.source, Math.max(maxInput, edge.sourceHandle));
        maxOutputsMap.set(edge.target, Math.max(maxOutput, edge.targetHandle));
    }

    for (const node of g.nodes) {
        if (node.type === "Internal" || node.type === "External") {
            node.data.n_inputs = maxInputsMap.get(node.id) || 1;
            node.data.n_outputs = maxOutputsMap.get(node.id) || 1;
        }
    }
    return g;
}

export default placeGraph;
