import { useEffect } from "react";
import { useUpdateNodeInternals } from "reactflow";

type UpdatePortsProps = {
    drainUpdatePortCounts: () => string[];
};

function UpdatePorts({ drainUpdatePortCounts }: UpdatePortsProps) {
    const updateNodeInternals = useUpdateNodeInternals();
    useEffect(() => {
        const updatedPortCounts = drainUpdatePortCounts();
        if (updatedPortCounts.length > 0) {
            updateNodeInternals(updatedPortCounts);
        }
    }, [drainUpdatePortCounts, updateNodeInternals]);
    return <></>;
}

export default UpdatePorts;
