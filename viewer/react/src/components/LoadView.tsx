import React, { useState } from "react";
import { EventVariantDeserializeData } from "shared_types/types/shared_types";

export const GRAPH_FORMATS = ["portgraph", "tket"] as const;
export type GraphFormat = (typeof GRAPH_FORMATS)[number];

interface LoadViewProps {
  loadData: (data: string, format: GraphFormat) => void;
}

const LoadView: React.FC<LoadViewProps> = ({ loadData }) => {
  const [fileFormat, setFileFormat] = useState<GraphFormat>("portgraph");

  return (
    <section className="bg-white shadow-md rounded-lg p-8 m-5 w-full max-w-md">
      <div className="flex flex-row items-center space-x-4">
        <select
          className="flex-grow bg-white border border-gray-300 rounded-md py-2 px-3 focus:outline-none focus:ring-2 focus:ring-blue-500"
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
    </section>
  );
};

export default LoadView;
