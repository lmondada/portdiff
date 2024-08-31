import { useState } from "react";

import PortDiffViewer from "./PortDiffViewer";

import DragDivider from "./DragDivider";
import HierarchyViewer from "./HierarchyViewer";

import { HierarchyEdge } from "shared_types/types/shared_types";

interface MainViewProps {
  graph: string;
  graphType: "portgraph";
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

  if (graphType !== "portgraph") {
    throw new Error("Graph type is not portgraph");
  }

  return (
    <div style={{ display: "flex", width: "100vw", height: "100vh" }}>
      <div style={{ width: `${widthPercentage - 2}%`, height: "100%" }}>
        <PortDiffViewer graph={graph} />
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
