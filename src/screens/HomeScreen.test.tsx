import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { seedPersistedHandHistory } from "../test/persistenceFixtures";
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
    expect(
      screen
        .getByText(/saved hand-history summaries:/i)
        .closest("li")?.textContent,
    ).toContain("Saved hand-history summaries: 0");
    expect(screen.queryByRole("link", { name: "Internal Tools" })).toBeNull();
  });

  it("shows the debug-only entry when enabled", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: true });

    renderWithProviders(<HomeScreen bootstrap={bootstrap} />, { bootstrap });

    expect(screen.getByRole("link", { name: "Internal Tools" })).toBeTruthy();
  });

  it("shows the saved hand-history count from cached storage", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    seedPersistedHandHistory(bootstrap.storageNamespace, [
      {
        handNumber: 3,
        summary: "Alice won 300 chip(s).",
        potTotal: 300,
        winningPlayers: ["Alice"],
        eliminatedPlayers: [],
        boardCards: [],
      },
      {
        handNumber: 4,
        summary: "Bob won 220 chip(s).",
        potTotal: 220,
        winningPlayers: ["Bob"],
        eliminatedPlayers: [],
        boardCards: [],
      },
    ]);

    renderWithProviders(<HomeScreen bootstrap={bootstrap} />, { bootstrap });

    expect(
      screen
        .getByText(/saved hand-history summaries:/i)
        .closest("li")?.textContent,
    ).toContain("Saved hand-history summaries: 2");
  });
});
