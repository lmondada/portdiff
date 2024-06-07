import { useUpdateNodeInternals } from "reactflow";

type UpdatePortsProps = {
    updatedPortCounts: string[];
    resetUpdatedPortCounts: () => void;
};

function UpdatePorts({
    updatedPortCounts,
    resetUpdatedPortCounts,
}: UpdatePortsProps) {
    const updateNodeInternals = useUpdateNodeInternals();
    if (updatedPortCounts.length > 0) {
        updateNodeInternals(updatedPortCounts);
        resetUpdatedPortCounts();
    }
    return <></>;
}

export default UpdatePorts;
