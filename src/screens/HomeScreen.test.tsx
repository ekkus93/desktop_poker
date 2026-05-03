import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { HomeScreen } from "./HomeScreen";

describe("HomeScreen", () => {
  it("renders the production entry points", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<HomeScreen bootstrap={bootstrap} />, { bootstrap });

    expect(
      screen.getByRole("heading", { level: 2, name: "Desktop Poker" }),
    ).toBeTruthy();
    expect(screen.getByRole("link", { name: "Host Tournament" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Join Tournament" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Rules" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Settings" })).toBeTruthy();
    expect(screen.queryByRole("link", { name: "Internal Tools" })).toBeNull();
  });

  it("shows the debug-only entry when enabled", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: true });

    renderWithProviders(<HomeScreen bootstrap={bootstrap} />, { bootstrap });

    expect(screen.getByRole("link", { name: "Internal Tools" })).toBeTruthy();
  });
});
