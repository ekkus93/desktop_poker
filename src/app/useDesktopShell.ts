import { useContext } from "react";
import { DesktopShellContext } from "./DesktopShellProvider";

export function useDesktopShell() {
  const context = useContext(DesktopShellContext);

  if (!context) {
    throw new Error("useDesktopShell must be used inside the shell provider.");
  }

  return context;
}
