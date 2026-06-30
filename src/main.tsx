import React, { lazy, Suspense } from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { resolveLayoutProbeSurface } from "./app/runtimeGate";

const layoutProbe = resolveLayoutProbeSurface(
  window.location.search,
  import.meta.env.DEV,
);

// Only load the probe bundle when running in dev mode with the probe surface
// active.  The dynamic import is evaluated lazily so the LayoutProbeApp module
// is never included in the production build.
const LayoutProbeApp = layoutProbe
  ? lazy(() =>
      import("./probe/LayoutProbeApp").then((m) => ({
        default: (props: React.ComponentProps<typeof m.LayoutProbeApp>) =>
          React.createElement(m.LayoutProbeApp, props),
      }))
    )
  : null;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {layoutProbe && LayoutProbeApp ? (
      <Suspense fallback={null}>
        <LayoutProbeApp surface={layoutProbe} />
      </Suspense>
    ) : (
      <App />
    )}
  </React.StrictMode>,
);
