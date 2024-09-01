import { useState } from "react";

import PortgraphViewer from "./PortgraphViewer";
import CircuitViewer from "./CircuitViewer";

import DragDivider from "./DragDivider";
import HierarchyViewer from "./HierarchyViewer";

import { HierarchyEdge } from "shared_types/types/shared_types";
import { GraphFormat } from "./LoadView";

interface MainViewProps {
  graph: string;
  graphType: GraphFormat;
  hierarchy: HierarchyEdge[];
  selected: number[];
  setSelected: (selected: number[]) => void;
}

const MainView: React.FC<MainViewProps> = ({
  graph,
  graphType,
  hierarchy,
  selected,
  setSelected,
}) => {
  const [widthPercentage, setWidthPercentage] = useState(70);

  const renderGraph = () => {
    switch (graphType) {
      case "portgraph":
        return <PortgraphViewer graph={graph} />;
      case "tket":
        return <CircuitViewer graph={graph} />;
      default:
        throw new Error(`Unsupported graph type: ${graphType}`);
    }
  };

  return (
    <div style={{ display: "flex", width: "100vw", height: "100vh" }}>
      <div style={{ width: `${widthPercentage - 2}%`, height: "100%" }}>
        {renderGraph()}
      </div>
      <DragDivider
        widthPercentage={widthPercentage}
        setWidthPercentage={setWidthPercentage}
      />
      <div
        style={{
          width: `${100 - widthPercentage - 2}%`,
          height: "100%",
        }}
      >
        <HierarchyViewer
          hierarchy={hierarchy}
          selected={selected}
          setSelected={setSelected}
        />
      </div>
    </div>
  );
};

export default MainView;
