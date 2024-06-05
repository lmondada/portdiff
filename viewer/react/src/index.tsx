import React from "react";
import ReactDOM from "react-dom";

import App from "./App";

import { createRoot } from "react-dom/client";

/**
 * Tells React what to render and where to render it.
 *
 * In our case, we're rendering our root `App` component to the DOM element with
 * the id of `root` in the `public/index.html` file.
 */
const container = document.getElementById("root");
const root = createRoot(container!);
root.render(<App />);
