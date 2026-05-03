import { useContext } from "react";
import { DesktopBootstrapContext } from "./DesktopBootstrapProvider";

export function useDesktopBootstrap() {
  const context = useContext(DesktopBootstrapContext);

  if (!context) {
    throw new Error("useDesktopBootstrap must be used inside the provider.");
  }

  return context;
}
