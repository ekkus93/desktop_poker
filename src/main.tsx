import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { LayoutProbeApp } from "./probe/LayoutProbeApp";

const searchParams = new URLSearchParams(window.location.search);
const layoutProbe = searchParams.get("layout-probe");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {layoutProbe ? <LayoutProbeApp surface={layoutProbe} /> : <App />}
  </React.StrictMode>,
);
