import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { HomeScreen } from "./HomeScreen";
import type { DesktopBootstrapState } from "../api/desktop";

const bootstrap: DesktopBootstrapState = {
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
  profileDirectory: "/tmp/desktop-poker/test-instance",
  launchJoinPayload: null,
  parsedLaunchJoinPayload: null,
  launchJoinPayloadError: null,
  debugToolsEnabled: true,
  backendModules: [],
  screens: [],
};

describe("HomeScreen", () => {
  it("renders the primary host and join entry points", () => {
    render(
      <MemoryRouter>
        <HomeScreen bootstrap={bootstrap} />
      </MemoryRouter>,
    );

    expect(
      screen.getByRole("heading", { level: 2, name: "Desktop Poker" }),
    ).toBeTruthy();
    expect(screen.getByRole("link", { name: "Host Tournament" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Join Tournament" })).toBeTruthy();
  });
});
