import { Edge, XYPosition } from "reactflow";

export const node_type_name = "portdiff" as const;

import { init_app, rewrite, select_nodes, select_diffs, hierarchy, current_graph, expand_boundary } from "wasm";
import { CSSProperties } from "react";

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
export type PlacedWasmNode = WasmNode & { position: XYPosition, selected: boolean };
export interface WasmEdge {
    id: string;
    source: string;
    target: string;
    sourceHandle: number;
    targetHandle: number;
    style: string | undefined;
}
export interface PlacedWasmEdge {
    id: string;
    source: string;
    target: string;
    sourceHandle: string;
    targetHandle: string;
    style: CSSProperties | undefined;
}

export interface WasmGraph {
    nodes: WasmNode[];
    edges: WasmEdge[];
}

export interface PlacedWasmGraph {
    nodes: PlacedWasmNode[];
    edges: PlacedWasmEdge[];
}

function validate_graph(g: WasmGraph) {
    for (const node of g.nodes) {
        if (!node.id) {
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

export function initApp(): WasmGraph {
    console.log(`init_app()`);
    let g: WasmGraph = JSON.parse(init_app());
    validate_graph(g);
    return g;
}

export function rewriteGraph(edges: WasmEdge[]): WasmGraph {
    console.log(`rewrite("${JSON.stringify(edges)}")`);
    const res = rewrite(JSON.stringify(edges));
    const g = JSON.parse(res);
    console.log("=>", g);
    return g;
}

export function selectNodes(nodeIds: Set<string>): WasmGraph {
    console.log(`select_nodes("${JSON.stringify(Array.from(nodeIds))}")`);
    const res = select_nodes(JSON.stringify(Array.from(nodeIds)));
    const g = JSON.parse(res);
    console.log(" =>", g);
    return g;
}

export function getHierarchy(): [string, string][] {
    return JSON.parse(hierarchy());
}

export function selectDiffs(diffIds: Set<string>) {
    console.log(`select_diffs("${JSON.stringify(Array.from(diffIds))}")`);
    select_diffs(JSON.stringify(Array.from(diffIds)));
}

export function currentGraph(): WasmGraph {
    return JSON.parse(current_graph());
}

export function expandBoundary(boundaryNode: string) {
    expand_boundary(boundaryNode);
}