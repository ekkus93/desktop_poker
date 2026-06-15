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
      screen.getByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();
    expect(screen.getByRole("link", { name: "Host Tournament" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Join Tournament" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Help" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Settings" })).toBeTruthy();
    expect(
      screen.queryByRole("heading", { level: 3, name: "Resume" }),
    ).toBeNull();
    expect(screen.queryByRole("link", { name: "Internal Tools" })).toBeNull();
  });

  it("keeps the home screen focused even when debug tools are enabled", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: true });

    renderWithProviders(<HomeScreen bootstrap={bootstrap} />, { bootstrap });

    expect(screen.queryByRole("link", { name: "Internal Tools" })).toBeNull();
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

    expect(screen.getByText("2 hands")).toBeTruthy();
    expect(
      screen.getByRole("heading", { level: 3, name: "Resume" }),
    ).toBeTruthy();
  });

  it("surfaces unreadable startup storage as a recovery card", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    localStorage.setItem(
      `${bootstrap.storageNamespace}:display-name`,
      "{bad-json",
    );

    renderWithProviders(<HomeScreen bootstrap={bootstrap} />, { bootstrap });

    expect(screen.getByText("Storage needs attention")).toBeTruthy();
    expect(
      screen.getByText(
        /some saved local preferences were unreadable and were reset to safe defaults/i,
      ),
    ).toBeTruthy();
    expect(screen.getByRole("link", { name: "Review" })).toBeTruthy();
  });

  it("shows all startup warnings when multiple storage reads fail (B2)", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    // Corrupt two separate storage keys to trigger two distinct warnings
    localStorage.setItem(
      `${bootstrap.storageNamespace}:display-name`,
      "{bad-json",
    );
    localStorage.setItem(
      `${bootstrap.storageNamespace}:hand-history-summaries`,
      "{bad-json",
    );

    renderWithProviders(<HomeScreen bootstrap={bootstrap} />, { bootstrap });

    expect(screen.getByText("Storage needs attention")).toBeTruthy();
    expect(
      screen.getByText(
        /some saved local preferences were unreadable and were reset to safe defaults/i,
      ),
    ).toBeTruthy();
    expect(
      screen.getByText(
        /saved hand history was unreadable and has been ignored/i,
      ),
    ).toBeTruthy();
  });
});
