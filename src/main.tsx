import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

async function bootstrap(): Promise<void> {
  let rootElement: React.ReactElement;

  if (import.meta.env.DEV) {
    const { resolveLayoutProbeSurface } = await import("./app/runtimeGate");
    const surface = resolveLayoutProbeSurface(window.location.search, true);
    if (surface) {
      const { LayoutProbeApp } = await import("./probe/LayoutProbeApp");
      rootElement = <LayoutProbeApp surface={surface} />;
    } else {
      rootElement = <App />;
    }
  } else {
    rootElement = <App />;
  }

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>{rootElement}</React.StrictMode>,
  );
}

bootstrap();
