import useCircuitViewer from "@/circuit_viewer";
import React, { useRef } from "react";

interface CircuitViewerProps {
  graph: string;
}

const CircuitViewer: React.FC<CircuitViewerProps> = ({ graph }) => {
  const containerRef = useRef<HTMLDivElement>(null);

  useCircuitViewer(containerRef, graph);

  return <div ref={containerRef} style={{ width: "100%", height: "100%" }} />;
};

export default CircuitViewer;
