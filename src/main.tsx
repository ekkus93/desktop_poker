import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { resolveLayoutProbeSurface } from "./app/runtimeGate";
import { LayoutProbeApp } from "./probe/LayoutProbeApp";

const layoutProbe = resolveLayoutProbeSurface(window.location.search, import.meta.env.DEV);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {layoutProbe ? <LayoutProbeApp surface={layoutProbe} /> : <App />}
  </React.StrictMode>,
);
