import type { ReactElement } from "react";
import { render } from "@testing-library/react";
import { MemoryRouter, type MemoryRouterProps } from "react-router-dom";
import type { DesktopBootstrapState } from "../api/desktop";
import { DesktopShellProvider } from "../app/DesktopShellProvider";

export function createBootstrap(
  overrides: Partial<DesktopBootstrapState> = {},
): DesktopBootstrapState {
  return {
    appName: "Desktop Poker",
    protocolVersion: 1,
    defaultHostPort: 43818,
    frontendStack: "React + TypeScript",
    serializationStrategy: "serde + canonical JSON bytes",
    framingStrategy: "length-prefixed JSON envelopes",
    joinPayloadEncoding: "pkr1_ compact join payload",
    runtimeTransport: "raw TCP over LAN",
    cryptoStack: ["ed25519-dalek", "x25519-dalek", "chacha20poly1305"],
    instanceId: "test-instance",
    instanceLabel: "test-instance",
    storageNamespace: "desktop-poker:test-instance",
    sessionIdentity: "desktop-session:test-instance",
    reconnectNamespace: "desktop-reconnect:test-instance",
    profileDirectory: "/home/test/desktop-poker/profiles/test-instance",
    launchJoinPayload: null,
    parsedLaunchJoinPayload: null,
    launchJoinPayloadError: null,
    debugToolsEnabled: true,
    llmApiKeyConfigured: false,
    llmProviderType: null,
    providerConfigError: null,
    backendModules: [],
    screens: [],
    ...overrides,
  };
}

export function renderWithProviders(
  ui: ReactElement,
  {
    bootstrap = createBootstrap(),
    initialEntries = ["/"],
  }: {
    bootstrap?: DesktopBootstrapState;
    initialEntries?: MemoryRouterProps["initialEntries"];
  } = {},
) {
  return render(
    <MemoryRouter initialEntries={initialEntries}>
      <DesktopShellProvider bootstrap={bootstrap}>{ui}</DesktopShellProvider>
    </MemoryRouter>,
  );
}
