import { useEffect } from "react";
import { Edge, useUpdateNodeInternals } from "reactflow";
import { WasmNode } from "../wasm_api";

/**
 * Custom hook to update the ports of a node in the React Flow diagram.
 * 
 * This hook ensures that there is always at least one free input and output port.
 */
function useUpdatePorts(
    nodes: WasmNode[],
    edges: Edge[],
    { addInputPort, addOutputPort }: { addInputPort: (id: string) => void, addOutputPort: (id: string) => void }
) {
    const updateNodeInternals = useUpdateNodeInternals();

    // Update rendering when number of ports changes
    useEffect(() => {
        for (const node of nodes) {
            updateNodeInternals(node.id);
        }
    }, [nodes, updateNodeInternals]);

    useEffect(() => {
        for (const node of nodes) {
            const outPorts = edges.filter(edge => edge.source === node.id).map(edge => edge.sourceHandle);
            const inPorts = edges.filter(edge => edge.target === node.id).map(edge => edge.targetHandle);
            const uniqueOutPorts = new Set(outPorts).size;
            const uniqueInPorts = new Set(inPorts).size;
            let updated = false;
            if (node.type !== "Boundary" && uniqueOutPorts >= node.data.n_outputs) {
                addOutputPort(node.id);
            }
            if (node.type !== "Boundary" && uniqueInPorts >= node.data.n_inputs) {
                addInputPort(node.id);
            }
        }
    },
        [nodes, edges]
    );
}

export default useUpdatePorts;

