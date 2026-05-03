import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type BackendModuleDescriptor = {
  name: string;
  responsibility: string;
};

export type ScreenDescriptor = {
  id: string;
  title: string;
  route: string;
  surface: string;
};

export type DesktopBootstrapState = {
  appName: string;
  protocolVersion: number;
  defaultHostPort: number;
  frontendStack: string;
  serializationStrategy: string;
  framingStrategy: string;
  joinPayloadEncoding: string;
  runtimeTransport: string;
  cryptoStack: string[];
  instanceId: string;
  profileDirectory: string;
  launchJoinPayload: string | null;
  debugToolsEnabled: boolean;
  backendModules: BackendModuleDescriptor[];
  screens: ScreenDescriptor[];
};

const BOOTSTRAP_EVENT = "desktop://bootstrap";

export function fetchBootstrapState() {
  return invoke<DesktopBootstrapState>("get_bootstrap_state");
}

export function subscribeBootstrap(
  onBootstrap: (bootstrap: DesktopBootstrapState) => void,
) {
  return listen<DesktopBootstrapState>(BOOTSTRAP_EVENT, (event) => {
    onBootstrap(event.payload);
  });
}
