"use client";

import type { NextPage } from "next";
import Head from "next/head";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import "@xyflow/react/dist/style.css";
import "@/styles/node_types.css";
import "@/styles/globals.css";

import init_core from "shared/shared";
import {
  EventVariantDeserializeData,
  EventVariantSetSelected,
  ViewModelVariantNone,
  ViewModelVariantLoaded,
  ViewModel,
} from "shared_types/types/shared_types";

import { update } from "./core";
import LoadView, { GRAPH_FORMATS, GraphFormat } from "@/components/LoadView";
import MainView from "@/components/MainView";

const Home: NextPage = () => {
  const [view, setView] = useState<ViewModel>(new ViewModelVariantNone());

  const callbacks = useMemo(
    () => ({
      setView,
      logInfo: console.log,
      logError: console.error,
    }),
    [setView]
  );

  const loadData = useCallback(
    (data: string, format: GraphFormat) => {
      update(new EventVariantDeserializeData(data, format), callbacks);
    },
    [callbacks]
  );

  const initialized = useRef(false);
  useEffect(() => {
    if (!initialized.current) {
      initialized.current = true;

      init_core().then(() => {
        const urlParams = new URLSearchParams(window.location.search);
        const type = urlParams.get("type");
        const data = urlParams.get("data");

        if (type && data && (type === "portgraph" || type === "tket")) {
          try {
            const decodedData = decodeURIComponent(data);
            const parsedData = JSON.parse(decodedData);
            loadData(JSON.stringify(parsedData), type);
          } catch (error) {
            console.error("Error parsing URL data:", error);
            update(new EventVariantDeserializeData("", "portgraph"), callbacks);
          }
        } else {
          update(new EventVariantDeserializeData("", "portgraph"), callbacks);
        }
      });
    }
  }, [callbacks, loadData]);

  const setSelected = useCallback(
    (selected: number[]) => {
      console.log("setting selected to", selected);
      update(new EventVariantSetSelected(selected), callbacks);
    },
    [callbacks]
  );

  if (view instanceof ViewModelVariantLoaded) {
    if (!GRAPH_FORMATS.includes(view.graph_type as any)) {
      throw new Error("Graph type is not supported");
    }
    console.log("hierarchy at main", view.hierarchy);
    console.log("selected at main", view.selected);
  }

  return (
    <>
      <Head>
        <title>PortGraph Diff Viewer</title>
      </Head>

      <main>
        {view instanceof ViewModelVariantLoaded ? (
          <MainView
            graph={view.graph}
            graphType={view.graph_type as "portgraph" | "tket"}
            hierarchy={view.hierarchy}
            hierarchyNodeLabels={view.hierarchy_node_labels}
            selected={view.selected}
            setSelected={setSelected}
          />
        ) : (
          <LoadView loadData={loadData} />
        )}
      </main>
    </>
  );
};

export default Home;
