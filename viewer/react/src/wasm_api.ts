import { Edge, XYPosition } from "reactflow";

export const node_type_name = "portdiff" as const;

import { graph } from "wasm";

export interface InternalNodeData {
    port_diff_id: string;
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
    position: XYPosition;
    data: InternalNodeData;
    type: "Internal";
}

export interface BoundaryNode {
    id: string;
    position: XYPosition;
    data: {},
    type: "Boundary";
}

export interface ExternalNode {
    id: string;
    position: XYPosition;
    data: ExternalNodeData;
    type: "External";
}

export type Node = InternalNode | BoundaryNode | ExternalNode;

export interface Graph {
    nodes: Node[];
    edges: Edge[];
}

function validate_graph(g: Graph) {
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

export function createGraph(): Graph {
    const g = JSON.parse(graph());
    validate_graph(g);
    return g;
}
