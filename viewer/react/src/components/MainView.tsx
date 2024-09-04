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
  hierarchyNodeLabels: string[];
  selected: number[];
  setSelected: (selected: number[]) => void;
}

const MainView: React.FC<MainViewProps> = ({
  graph,
  graphType,
  hierarchy,
  hierarchyNodeLabels,
  selected,
  setSelected,
}) => {
  const [heightPercentage, setHeightPercentage] = useState(70);

  const renderGraph = () => {
    switch (graphType) {
      case "portgraph":
        return <PortgraphViewer graph={graph} />;
      case "tket":
        return <CircuitViewer circuitJson={graph} />;
      default:
        throw new Error(`Unsupported graph type: ${graphType}`);
    }
  };

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100vw",
        height: "100vh",
      }}
    >
      <div style={{ height: `${heightPercentage - 2}%`, width: "100%" }}>
        {renderGraph()}
      </div>
      <DragDivider
        heightPercentage={heightPercentage}
        setHeightPercentage={setHeightPercentage}
      />
      <div
        style={{
          height: `${100 - heightPercentage - 2}%`,
          width: "100%",
        }}
      >
        <HierarchyViewer
          hierarchy={hierarchy}
          hierarchyNodeLabels={hierarchyNodeLabels}
          selected={selected}
          setSelected={setSelected}
        />
      </div>
    </div>
  );
};

export default MainView;
