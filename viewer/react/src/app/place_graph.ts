import { XYPosition, Node, Edge } from "@xyflow/react";
import Dagre from "@dagrejs/dagre";
import { RFGraph, RFNode } from "shared_types/types/shared_types";
import { Seq } from "shared_types/serde/types";

export function computePositions<
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

export function placeGraph(g: RFGraph): { nodes: Node[], edges: Edge[] } {
    const unplaced_nodes = g.nodes.map((n) => ({ id: n.id, data: getNodeData(n), type: "custom" }));
    const edges = g.edges.map(({ source, target, sourceHandle, targetHandle }, i) => ({
        id: `${source}-${target}-${i}`,
        type: "step",
        source: source,
        sourceHandle: `source${sourceHandle}`,
        target: target,
        targetHandle: `target${targetHandle}`,
    }));
    const positions = computePositions({ nodes: unplaced_nodes, edges });
    const nodes = unplaced_nodes.map((node) => ({
        ...node,
        position: positions[node.id],
    }));
    return {
        nodes,
        edges,
    };
}

function getNodeData(n: RFNode): { numInHandles: number, numOutHandles: number, label: string } {
    return {
        numInHandles: n.numInHandles,
        numOutHandles: n.numOutHandles,
        label: n.id,
    };
}


export default placeGraph;


