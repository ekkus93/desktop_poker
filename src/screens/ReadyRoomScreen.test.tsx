import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { ReadyRoomScreen } from "./ReadyRoomScreen";

describe("ReadyRoomScreen", () => {
  it("requires all visible participants to be ready before the start link appears", () => {
    const bootstrap = createBootstrap({
      debugToolsEnabled: true,
      launchJoinPayload: "pkr1_join",
    });

    renderWithProviders(<ReadyRoomScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/ready-room"],
    });

    expect(screen.getByText("Pending seats use host-controlled ready toggles in debug mode.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Start tournament" }).hasAttribute("disabled")).toBe(true);
    expect(screen.getByRole("button", { name: "Not ready" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Host marks ready" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Not ready" }));
    fireEvent.click(screen.getByRole("button", { name: "Host marks ready" }));

    expect(screen.getByRole("button", { name: "Ready" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Host marked ready" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Start tournament" })).toBeNull();
    expect(screen.getByRole("link", { name: "Start tournament" }).getAttribute("href")).toBe("/table");
    expect(screen.queryByText("All seats must be ready.")).toBeNull();
  });

  it("keeps remote readiness passive outside debug mode and lets the leave flow close cleanly", () => {
    const bootstrap = createBootstrap({
      debugToolsEnabled: false,
      launchJoinPayload: "pkr1_join",
    });

    renderWithProviders(<ReadyRoomScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/ready-room"],
    });

    expect(screen.queryByText("Pending seats use host-controlled ready toggles in debug mode.")).toBeNull();
    expect(screen.queryByRole("button", { name: "Host marks ready" })).toBeNull();
    expect(screen.getByText("Waiting on player readiness.")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Leave table" }));

    expect(screen.getByText("Leave before start?")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Leave table" }).getAttribute("href")).toBe("/");

    fireEvent.click(screen.getByRole("button", { name: "Stay ready" }));

    expect(screen.queryByText("Leave before start?")).toBeNull();
  });
});