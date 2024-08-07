"use client";

import type { NextPage } from "next";
import Head from "next/head";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import "@xyflow/react/dist/style.css";
import "@/app/node_types.css";

import init_core from "shared/shared";
import {
  EventVariantDeserializeData,
  EventVariantSetSelected,
  ViewModelVariantNone,
  ViewModelVariantLoaded,
  ViewModel,
} from "shared_types/types/shared_types";

import { update } from "./core";
import LoadView from "@/components/LoadView";
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

  const initialized = useRef(false);
  useEffect(
    () => {
      if (!initialized.current) {
        initialized.current = true;

        init_core().then(() => {
          const data = `{
  "sources": [
    0
  ],
  "sinks": [
    1
  ],
  "all_nodes": [
    {
      "value": {
        "graph": {
          "node_meta": [
            {
              "n": {
                "first_port": 0,
                "incoming": 1,
                "outgoing": 3,
                "capacity": 3
              }
            },
            {
              "n": {
                "first_port": 3,
                "incoming": 4,
                "outgoing": 1,
                "capacity": 4
              }
            },
            {
              "n": {
                "first_port": 7,
                "incoming": 2,
                "outgoing": 3,
                "capacity": 4
              }
            },
            {
              "n": {
                "first_port": 11,
                "incoming": 4,
                "outgoing": 0,
                "capacity": 3
              }
            }
          ],
          "port_link": [
            3,
            4,
            5,
            0,
            1,
            2,
            7,
            6,
            11,
            12,
            13,
            8,
            9,
            10
          ],
          "port_meta": [
            2147483649,
            2147483649,
            2147483649,
            2,
            2,
            2,
            2147483650,
            3,
            2147483651,
            2147483651,
            2147483651,
            4,
            4,
            4
          ],
          "node_free": null,
          "port_free": [],
          "node_count": 4,
          "port_count": 14,
          "link_count": 7
        },
        "boundary": []
      },
      "incoming": []
    },
    {
      "value": {
        "graph": {
          "node_meta": [
            {
              "n": {
                "first_port": 0,
                "incoming": 4,
                "outgoing": 0,
                "capacity": 3
              }
            },
            {
              "n": {
                "first_port": 3,
                "incoming": 1,
                "outgoing": 3,
                "capacity": 3
              }
            }
          ],
          "port_link": [
            null,
            null,
            null,
            null,
            null,
            null
          ],
          "port_meta": [
            1,
            1,
            1,
            2147483650,
            2147483650,
            2147483650
          ],
          "node_free": null,
          "port_free": [],
          "node_count": 2,
          "port_count": 6,
          "link_count": 0
        },
        "boundary": [
          [
            {
              "node": 0,
              "port": {
                "Incoming": 0
              }
            },
            0
          ],
          [
            {
              "node": 0,
              "port": {
                "Incoming": 1
              }
            },
            0
          ],
          [
            {
              "node": 0,
              "port": {
                "Incoming": 2
              }
            },
            0
          ],
          [
            {
              "node": 1,
              "port": {
                "Outgoing": 0
              }
            },
            0
          ],
          [
            {
              "node": 1,
              "port": {
                "Outgoing": 1
              }
            },
            0
          ],
          [
            {
              "node": 1,
              "port": {
                "Outgoing": 2
              }
            },
            0
          ]
        ]
      },
      "incoming": [
        {
          "source": 0,
          "value": {
            "subgraph": {
              "nodes": [
                1,
                2
              ],
              "edges": [
                {
                  "outgoing": 0,
                  "node": 1
                }
              ]
            },
            "port_map": [
              [
                {
                  "Bound": {
                    "edge": {
                      "outgoing": 0,
                      "node": 0
                    },
                    "end": "Right"
                  }
                },
                0
              ],
              [
                {
                  "Bound": {
                    "edge": {
                      "outgoing": 0,
                      "node": 2
                    },
                    "end": "Left"
                  }
                },
                3
              ],
              [
                {
                  "Bound": {
                    "edge": {
                      "outgoing": 1,
                      "node": 0
                    },
                    "end": "Right"
                  }
                },
                1
              ],
              [
                {
                  "Bound": {
                    "edge": {
                      "outgoing": 1,
                      "node": 2
                    },
                    "end": "Left"
                  }
                },
                4
              ],
              [
                {
                  "Bound": {
                    "edge": {
                      "outgoing": 2,
                      "node": 0
                    },
                    "end": "Right"
                  }
                },
                2
              ],
              [
                {
                  "Bound": {
                    "edge": {
                      "outgoing": 2,
                      "node": 2
                    },
                    "end": "Left"
                  }
                },
                5
              ]
            ]
          }
        }
      ]
    }
  ]
}`;

          update(new EventVariantDeserializeData(data), callbacks);
          // update(new EventVariantDeserializeData(""), callbacks);
        });
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    /*once*/ []
  );

  const loadData = useCallback(
    (data: string) => {
      update(new EventVariantDeserializeData(data), callbacks);
    },
    [callbacks]
  );

  const setSelected = useCallback(
    (selected: number[]) => {
      console.log("setting selected to", selected);
      update(new EventVariantSetSelected(selected), callbacks);
    },
    [callbacks]
  );

  if (view instanceof ViewModelVariantLoaded) {
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
            hierarchy={view.hierarchy}
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
