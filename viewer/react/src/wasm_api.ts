import { Edge, XYPosition } from "reactflow";

export const node_type_name = "portdiff" as const;

import { init_app, rewrite } from "wasm";
import place_graph from "./place_graph";

export interface InternalNodeData {
    port_diff_id: string;
    color: string | undefined;
    label: string;
    n_inputs: number;
    n_outputs: number;
}

export interface ExternalNodeData {
    label: string;
    n_inputs: number;
    n_outputs: number;
}

export interface InternalNode {
    id: string;
    data: InternalNodeData;
    type: "Internal";
}

export interface BoundaryNode {
    id: string;
    data: {},
    type: "Boundary";
}

export interface ExternalNode {
    id: string;
    data: ExternalNodeData;
    type: "External";
}

export type WasmNode = InternalNode | BoundaryNode | ExternalNode;
export type PlacedWasmNode = WasmNode & { position: XYPosition };
export interface WasmEdge {
    id: string;
    source: string;
    target: string;
    sourceHandle: number;
    targetHandle: number;
}
export interface PlacedWasmEdge {
    id: string;
    source: string;
    target: string;
    sourceHandle: string;
    targetHandle: string;
}

export interface WasmGraph {
    nodes: WasmNode[];
    edges: WasmEdge[];
}

export interface PlacedWasmGraph {
    nodes: PlacedWasmNode[];
    edges: PlacedWasmEdge[];
}

function validate_graph(g: PlacedWasmGraph) {
    for (const node of g.nodes) {
        if (!node.id || !node.position) {
            throw new Error("Invalid node");
        }
        switch (node.type) {
            case "Internal":
                if (!node.data || !node.data.port_diff_id) {
                    throw new Error("Invalid internal node");
                }
                break;
            case "Boundary":
                break;
            case "External":
                if (!node.data) {
                    throw new Error("Invalid external node");
                }
                break;
        }
    }
}

export function initApp(): PlacedWasmGraph {
    let g: WasmGraph = JSON.parse(init_app());
    let placed_g = place_graph(g);
    validate_graph(placed_g);
    return placed_g;
}

export function rewriteGraph(edges: WasmEdge[]): WasmGraph {
    const res = rewrite(JSON.stringify(edges));
    const g = JSON.parse(res);
    return g;
}


