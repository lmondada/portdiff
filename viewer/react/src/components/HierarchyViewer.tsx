import {
  Dispatch,
  SetStateAction,
  useCallback,
  useEffect,
  useState,
} from "react";
import {
  ReactFlow,
  Node,
  Edge,
  NodeChange,
  applyNodeChanges,
  NodeTypes,
  Handle,
  Position,
} from "@xyflow/react";
import { HierarchyEdge } from "shared_types/types/shared_types";
import { computePositions } from "@/app/place_graph";

type HierarchyViewerProps = {
  hierarchy: HierarchyEdge[];
  selected: number[];
  setSelected: (selected: number[]) => void;
};

class SelectedNodesTracker {
  private oldSelected: Set<number>;
  private newSelected: Set<number>;

  constructor(initialSelected: number[]) {
    this.oldSelected = new Set(initialSelected);
    this.newSelected = new Set(initialSelected);
  }

  updateNodes(nodes: Node[]): SelectedNodesTracker {
    const newTracker = new SelectedNodesTracker([]);
    newTracker.oldSelected = this.oldSelected;
    newTracker.newSelected = new Set(
      nodes.filter((node) => node.selected).map((node) => parseInt(node.id))
    );
    return newTracker;
  }

  getNewSelected(): Set<number> {
    return this.newSelected;
  }

  hasSelectionChanged(): boolean {
    return !setsEqual(this.oldSelected, this.newSelected);
  }
}

type State = {
  nodes: Node[];
  edges: Edge[];
  selectedNodesTracker: SelectedNodesTracker;
};

function getStateSetters(setState: Dispatch<SetStateAction<State | null>>): {
  setNodes: (update: (nodes: State["nodes"]) => State["nodes"]) => void;
  setSelectedNodesTracker: (
    update: (
      nodes: State["nodes"],
      tracker: State["selectedNodesTracker"]
    ) => State["selectedNodesTracker"]
  ) => void;
} {
  return {
    setNodes: (update: (nodes: State["nodes"]) => State["nodes"]) => {
      setState((state) => {
        if (state === null) {
          return state;
        }
        return {
          ...state,
          nodes: update(state.nodes),
        };
      });
    },
    setSelectedNodesTracker: (
      update: (
        nodes: State["nodes"],
        tracker: State["selectedNodesTracker"]
      ) => State["selectedNodesTracker"]
    ) => {
      setState((state) => {
        if (state === null) {
          return state;
        }
        return {
          ...state,
          selectedNodesTracker: update(state.nodes, state.selectedNodesTracker),
        };
      });
    },
  };
}

function HierarchyViewer({
  hierarchy,
  selected,
  setSelected,
}: HierarchyViewerProps) {
  const [state, setState] = useState<null | State>(null);

  const { setNodes, setSelectedNodesTracker } = getStateSetters(setState);

  // Reset state every time we get new props
  useEffect(() => {
    const unplaced_nodes = Array.from(
      new Set(
        hierarchy
          .map(({ parent, child }) => [parent.toString(), child.toString()])
          .flat()
      )
    ).map((id) => ({ id, data: {}, type: "custom" }));
    const edges = hierarchy.map(({ parent, child }) => ({
      id: `${parent}-${child}`,
      type: "step",
      source: parent.toString(),
      target: child.toString(),
    }));
    const positions = computePositions({ nodes: unplaced_nodes, edges });
    const selectedSet = new Set(selected);
    const nodes = unplaced_nodes.map((node) => ({
      ...node,
      position: positions[node.id],
      selected: selectedSet.has(parseInt(node.id)),
    }));
    setState({
      nodes,
      edges,
      selectedNodesTracker: new SelectedNodesTracker(selected),
    });
  }, [hierarchy, selected]);

  // Watch changes to selected nodes...
  useEffect(() => {
    setSelectedNodesTracker((prevNodes, prevTracker) => {
      return prevTracker.updateNodes(prevNodes);
    });
  }, [state?.nodes]);

  // ...and report them
  useEffect(() => {
    if (state === null) {
      return;
    }
    const selectedNodesTracker = state.selectedNodesTracker;

    // Check if the selected nodes have changed
    if (selectedNodesTracker.hasSelectionChanged()) {
      console.log("updating selection");
      setSelected(Array.from(selectedNodesTracker.getNewSelected()));
    } else {
      console.log("no selection change");
    }
  }, [state?.selectedNodesTracker, setSelected]);

  const viewHandlers = useGraphHandlers(setNodes);

  return (
    <div style={{ height: "100%", width: "100%" }}>
      {state && (
        <ReactFlow
          nodes={state.nodes}
          edges={state.edges}
          nodeTypes={hierarchyNodeTypes}
          {...viewHandlers}
        />
      )}
    </div>
  );
}

function setsEqual(a: Set<number>, b: Set<number>) {
  return a.size === b.size && Array.from(a).every((id) => b.has(id));
}

export default HierarchyViewer;

export function useGraphHandlers(
  setNodes: (nodesUpdate: (nodes: Node[]) => Node[]) => void
) {
  const onNodesChange = useCallback(
    (changes: NodeChange[]) => {
      const allowedChanges = changes.filter(
        (change) => change.type === "select" || change.type === "position"
      );
      if (allowedChanges.length > 0) {
        setNodes((nds) => {
          if (!nds) {
            return nds;
          }
          return applyNodeChanges(allowedChanges, nds);
        });
      }
    },
    [setNodes]
  );
  return { onNodesChange };
}

const hierarchyNodeTypes: NodeTypes = {
  custom: HierarchyNodeViewer,
};

function HierarchyNodeViewer({}: { id: string }) {
  let className = `node rounded-full w-8 h-8 bg-white border border-black `;
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
