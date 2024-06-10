import {
    Handle,
    Position,
    Node,
    NodeTypes,
    useUpdateNodeInternals,
} from "reactflow";
import "./node_types.css";
import { ExternalNodeData, InternalNodeData } from "../../wasm_api";
import { useEffect, useLayoutEffect, useState } from "react";

export const nodeTypes: NodeTypes = {
    Internal: InternalNodeViewer,
    External: ExternalNodeViewer,
    Boundary: BoundaryNodeViewer,
};

function BoundaryNodeViewer() {
    return (
        <div>
            <Handle
                type="source"
                position={Position.Bottom}
                className="handle handle-top"
            />
            <div className="boundary-node"></div>
            <Handle
                type="target"
                position={Position.Top}
                className="handle handle-bottom"
            />
        </div>
    );
}

function InternalNodeViewer({ data }: { data: InternalNodeData }) {
    return <NodeViewer data={data} type="Internal" />;
}

function ExternalNodeViewer({ data }: { data: ExternalNodeData }) {
    return <NodeViewer data={data} type="External" />;
}

function simpleHash(str: string): number {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        const char = str.charCodeAt(i);
        hash = (hash << 5) - hash + char;
        hash |= 0; // Convert to 32bit integer
    }
    return hash >>> 0;
}

function NodeViewer({
    id,
    data,
    type,
}: {
    id: string;
    data: InternalNodeData | ExternalNodeData;
    type: "Internal" | "External";
}) {
    const [oldArity, setOldArity] = useState<{
        n_inputs: number;
        n_outputs: number;
    } | null>(null);
    const updateNodeInternals = useUpdateNodeInternals();

    // useLayoutEffect(() => {
    //     if (
    //         oldArity !== null &&
    //         (data.n_inputs !== oldArity.n_inputs ||
    //             data.n_outputs !== oldArity.n_outputs)
    //     ) {
    //         setOldArity({ n_inputs: data.n_inputs, n_outputs: data.n_outputs });
    //         updateNodeInternals(id);
    //     }
    // }, [data.n_inputs, data.n_outputs, oldArity, id, updateNodeInternals]);

    let input_pos = Array.from({ length: data.n_inputs }).map((_, i) => ({
        left: i * 10 - 5 * (data.n_inputs - 1),
    }));
    let output_pos = Array.from({ length: data.n_outputs }).map((_, i) => ({
        left: i * 10 - 5 * (data.n_outputs - 1),
    }));
    let is_active = type === "Internal";
    let className = "node";
    className += is_active ? " active" : " inactive nodrag";
    if ("port_diff_id" in data && data.port_diff_id) {
        className += ` color-palette-${simpleHash(data.port_diff_id) % 9}`;
    }
    return (
        <div className={className}>
            {input_pos.map((pos, i) => (
                <Handle
                    type="target"
                    position={Position.Top}
                    id={"in" + i}
                    key={"in" + i}
                    style={{ left: `calc(50% + ${pos.left}px)` }}
                    className="handle handle-top"
                />
            ))}
            <div className="label">{data.label}</div>
            {output_pos.map((pos, i) => (
                <Handle
                    type="source"
                    position={Position.Bottom}
                    id={"out" + i}
                    key={"out" + i}
                    style={{ left: `calc(50% + ${pos.left}px)` }}
                    className="handle handle-bottom"
                />
            ))}
        </div>
    );
}

export default NodeViewer;

export const hierarchyNodeTypes: NodeTypes = {
    custom: HierarchyNodeViewer,
};

function HierarchyNodeViewer({ id }: { id: string }) {
    let className = `node active color-palette-${
        simpleHash(id) % 9
    } rounded-full w-8 h-8 `;
    return (
        <div className={className}>
            <Handle
                type="target"
                position={Position.Top}
                className="handle handle-top"
            />
            <Handle
                type="source"
                position={Position.Bottom}
                className="handle handle-bottom"
            />
        </div>
    );
}
