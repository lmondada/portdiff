import { useState } from "react";

import PortDiffViewer from "./PortDiffViewer";

import "tailwindcss/tailwind.css";
import "reactflow/dist/style.css";
import DragDivider from "./DragDivider";
import HierarchyViewer from "./HierarchyViewer";

import { HierarchyEdge, RFGraph } from "shared_types/types/shared_types";

interface MainViewProps {
  graph: RFGraph;
  hierarchy: HierarchyEdge[];
  selected: number[];
  setSelected: (selected: number[]) => void;
}

const MainView: React.FC<MainViewProps> = ({
  graph,
  hierarchy,
  selected,
  setSelected,
}) => {
  const [widthPercentage, setWidthPercentage] = useState(70);

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
