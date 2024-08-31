import {
  ReactFlow,
  Background,
  BackgroundVariant,
  SelectionMode,
} from "@xyflow/react";

import { useMemo } from "react";
import placeGraph from "@/app/place_graph";
import PortDiffNode from "./PortDiffNode";
import { RFGraph } from "shared_types/types/shared_types";

type PortDiffViewerProps = {
  graph: string;
};
function PortDiffViewer({ graph }: PortDiffViewerProps) {
  // const onNodesChangeSelectOnly = useCallback(
  //   (changes: NodeChange[]) => {
  //     const selectChanges = changes.filter(
  //       (change) => change.type === "select"
  //     );
  //     if (selectChanges.length !== 0) {
  //       setGraphState((g) =>
  //         g.applyNodesChange(selectChanges, setUpdatedPortCounts)
  //       );
  //     }
  //   },
  //   [graph, setGraphState, setUpdatedPortCounts]
  // );
  const viewHandlers = {
    onNodesChange: () => {},
    onEdgesChange: () => {},
    // onConnect: () => { },
    // onInit: () => { },
    // isValidConnection: () => true,
    // onDoubleClick: () => { },
  };

  const flowOpts = {
    nodesDraggable: true,
    selectionOnDrag: false,
    panOnDrag: true,
    zoomOnDoubleClick: false,
    selectionMode: "partial" as SelectionMode,
  };

  const bg = {
    variant: "dots" as BackgroundVariant,
    color: "#333",
  };

  const parsedGraph = useMemo(() => {
    try {
      return JSON.parse(graph) as RFGraph;
    } catch (error) {
      console.error("Failed to parse graph:", error);
      return null;
    }
  }, [graph]);

  const { nodes, edges } = useMemo(() => {
    if (parsedGraph) {
      return placeGraph(parsedGraph);
    } else {
      return { nodes: [], edges: [] };
    }
  }, [parsedGraph]);

  return (
    <div style={{ height: "100%", width: "100%" }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        {...viewHandlers}
        {...flowOpts}
      >
        <Background {...bg} />
      </ReactFlow>
    </div>
  );
}

const nodeTypes = {
  custom: PortDiffNode,
};

export default PortDiffViewer;
