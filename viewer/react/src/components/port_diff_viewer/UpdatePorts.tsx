import { Edge } from "reactflow";
import { create } from "mutative";
import useUpdatePorts from "../../hooks/useUpdatePorts";
import { Dispatch, SetStateAction, useCallback } from "react";
import { InternalNodeData, Node } from "../../wasm_api";

function UpdatePorts({
    nodes,
    edges,
    setNodes,
}: {
    nodes: Node[];
    edges: Edge[];
    setNodes: Dispatch<SetStateAction<Node[]>>;
}) {
    const addInputPort = useCallback(
        (id: string) => {
            setNodes((prevNodes) => {
                return prevNodes.map((node) => {
                    if (node.id === id && node.type !== "Boundary") {
                        return create(node, (draft) => {
                            draft.data.n_inputs = node.data.n_inputs + 1;
                        });
                    } else {
                        return node;
                    }
                });
            });
        },
        [nodes, edges, setNodes],
    );

    const addOutputPort = useCallback(
        (id: string) => {
            setNodes((prevNodes) => {
                return prevNodes.map((node) => {
                    if (node.id === id && node.type !== "Boundary") {
                        return create(node, (draft) => {
                            draft.data.n_outputs = node.data.n_outputs + 1;
                        });
                    } else {
                        return node;
                    }
                });
            });
        },
        [nodes, edges, setNodes],
    );

    useUpdatePorts(nodes, edges, { addInputPort, addOutputPort });
    return <></>;
}

export default UpdatePorts;
