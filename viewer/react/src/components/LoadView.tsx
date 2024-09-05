import { BASE_URL } from "@/config";
import React, { useState, useCallback, useRef, useEffect } from "react";

export const GRAPH_FORMATS = ["portgraph", "tket"] as const;
export type GraphFormat = (typeof GRAPH_FORMATS)[number];

interface LoadViewProps {
  loadData: (data: string, format: GraphFormat) => void;
}

interface TooltipProps {
  content: string;
  isVisible: boolean;
}

const Tooltip: React.FC<TooltipProps> = ({ content, isVisible }) => {
  const tooltipRef = useRef<HTMLDivElement>(null);
  const [leftOffset, setLeftOffset] = useState(0);

  useEffect(() => {
    if (isVisible && tooltipRef.current) {
      const rect = tooltipRef.current.getBoundingClientRect();
      if (rect.left < 0) {
        setLeftOffset(-rect.left);
      } else {
        setLeftOffset(0);
      }
    }
  }, [isVisible]);

  if (!isVisible) return null;

  return (
    <div
      ref={tooltipRef}
      className="absolute top-full left-1/2 transform -translate-x-1/2 mt-2 p-3 bg-gray-800 dark:bg-gray-200 text-white dark:text-gray-800 text-sm rounded-lg shadow-lg z-10"
      style={{
        maxWidth: "90vw",
        width: "400px",
        marginLeft: leftOffset,
      }}
    >
      <div className="relative">
        <div
          className="absolute top-0 left-1/2 transform -translate-x-1/2 -translate-y-full w-3 h-3 bg-gray-800 dark:bg-gray-200 rotate-45"
          style={{ marginLeft: -leftOffset }}
        ></div>
        {content}
      </div>
    </div>
  );
};

const ReadmeDescription: React.FC = () => {
  return (
    <div className="bg-white dark:bg-gray-800 shadow-md rounded-lg p-6 mt-6 w-full max-w-3xl">
      <h2 className="text-xl font-semibold mb-4 dark:text-white">
        About this Viewer
      </h2>
      <p className="mb-4 dark:text-gray-300">
        This viewer visualizes the search space of rewrites for optimisations on
        graph domains, and in particular quantum circuits. Crucially, the search
        space is not the space of rewrites, but a powerset of it: multiple
        rewrites can be combined and applied in parallel. The key contribution
        of this work is the efficient representation of this large space, as
        well as:
      </p>
      <ul className="list-disc pl-5 mb-4 dark:text-gray-300">
        <li>Fast pattern matching on the search space,</li>
        <li>Fast testing for compatibility of rewrites,</li>
        <li>
          Fast extraction of final results given a set of compatible rewrites.
        </li>
      </ul>
      <h2 className="text-xl font-semibold mb-4 dark:text-white">Usage</h2>
      <p className="dark:text-gray-300">
        Start by loading a file or selecting one of the provided examples. A
        hierarchy of rewrites will be loaded in the bottom pane, while the top
        pane shows the graph resulting from the application of the selected
        rewrites. Select one or several rewrites (nodes) in the hierarchy pane
        and observe the changes in the resulting graph. Use command/control to
        select multiple rewrites. Drag the mouse to move around the hierarchy
        pane. Some rewrites may not be combined, in which case trying to select
        them will not succeed.
      </p>
    </div>
  );
};

const LoadView: React.FC<LoadViewProps> = ({ loadData }) => {
  const [fileFormat, setFileFormat] = useState<GraphFormat>("portgraph");
  const [activeTooltip, setActiveTooltip] = useState<string | null>(null);

  const loadExampleFile = useCallback(
    async (filename: string) => {
      try {
        const response = await fetch(`${BASE_URL}/examples/${filename}`);
        const content = await response.text();
        loadData(content, "tket");
      } catch (error) {
        console.error("Error loading example file:", error);
      }
    },
    [loadData]
  );

  const exampleInfo = [
    {
      name: "small_demo",
      file: "small_demo.json",
      description: "The search space and rewrites for a small dummy circuit.",
    },
    {
      name: "barenco_tof_5_min",
      file: "barenco_tof_5_min.json",
      description:
        "Barenco decomposition of a 5-qubit Toffoli gate into the Clifford+T basis. Rewrites are squashed as they are selected to keep the rewriting space as flat and small as possible, making the resulting SAT problem easier.",
    },
    {
      name: "barenco_tof_5_full",
      file: "barenco_tof_5_full.json",
      description:
        "Barenco decomposition of a 5-qubit Toffoli gate into the Clifford+T basis. This is the uncompressed version of the first example, thus giving a better representation of the search space (rewrites not leading to CX decreases are still pruned). Using a SAT solver on this unsquashed space would be less effective.",
    },
  ];

  return (
    <div className="flex flex-col items-center justify-center min-h-screen bg-gray-100 dark:bg-gray-900 py-8">
      <section className="bg-white dark:bg-gray-800 shadow-md rounded-lg p-8 m-5 w-full max-w-md">
        <div className="flex flex-col space-y-4">
          <div className="flex flex-row items-center space-x-4">
            <select
              className="flex-grow bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md py-2 px-3 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:text-white"
              value={fileFormat}
              onChange={(e) => setFileFormat(e.target.value as GraphFormat)}
            >
              {GRAPH_FORMATS.map((format) => (
                <option key={format} value={format}>
                  {format.charAt(0).toUpperCase() + format.slice(1)}
                </option>
              ))}
            </select>
            <input
              type="file"
              id="fileInput"
              className="hidden"
              onChange={(e) => {
                const file = e.target.files?.[0];
                if (file) {
                  const reader = new FileReader();
                  reader.onload = (event) => {
                    const content = event.target?.result as string;
                    loadData(content, fileFormat);
                  };
                  reader.readAsText(file);
                }
              }}
            />
            <button
              className="bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline"
              onClick={() => document.getElementById("fileInput")?.click()}
            >
              Load File
            </button>
          </div>
          <div className="text-sm text-gray-600 dark:text-gray-400">
            Looking for examples? Try{" "}
            {exampleInfo.map((example, index) => (
              <React.Fragment key={example.name}>
                <span className="relative inline-block">
                  <button
                    className="text-blue-500 hover:underline dark:text-blue-400"
                    onClick={() => loadExampleFile(example.file)}
                    onMouseEnter={() => setActiveTooltip(example.name)}
                    onMouseLeave={() => setActiveTooltip(null)}
                  >
                    {example.name}
                  </button>
                  <Tooltip
                    content={example.description}
                    isVisible={activeTooltip === example.name}
                  />
                </span>
                {index < exampleInfo.length - 1 && ", "}
                {index === exampleInfo.length - 2 && "or "}
              </React.Fragment>
            ))}
            .
          </div>
        </div>
      </section>
      <ReadmeDescription />
    </div>
  );
};

export default LoadView;
