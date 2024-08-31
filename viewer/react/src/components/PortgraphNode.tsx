import { useEffect } from "react";
import { Handle, Position, useUpdateNodeInternals } from "@xyflow/react";

interface PortDiffNodeProps {
  id: string;
  data: {
    label: string;
    numInHandles: number;
    numOutHandles: number;
  };
}
const PortDiffNode = ({ id, data }: PortDiffNodeProps) => {
  //   const updateNodeInternals = useUpdateNodeInternals();

  //   useEffect(() => {
  //     updateNodeInternals(id);
  //   }, [data, updateNodeInternals]);

  const handleTypes = ["target", "source"] as const;
  type HandleType = (typeof handleTypes)[number];

  const genHandles = (type: HandleType, count: number) => {
    return [...Array(count)].map((_, index) => {
      const leftPos = index * 10 - 5 * (count - 1);
      return (
        <Handle
          key={index.toString()}
          type={type}
          position={type === "target" ? Position.Top : Position.Bottom}
          id={`${type}${index}`}
          style={{ left: `calc(50% + ${leftPos}px)` }}
          className="handle"
        />
      );
    });
  };

  return (
    <>
      {genHandles("target", data.numInHandles)}
      <div
        style={{
          padding: "10px 20px",
          backgroundColor: "white",
          border: "1px solid black",
        }}
      >
        {data.label}
      </div>
      {genHandles("source", data.numOutHandles)}
    </>
  );
};

export default PortDiffNode;
