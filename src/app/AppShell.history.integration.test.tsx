import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  createAppBootstrap,
  createTableViewSnapshot,
} from "../test/appIntegrationFixtures";
import { persistHandHistory } from "./persistence";

vi.mock("../api/desktop", async () => {
  const actual =
    await vi.importActual<typeof import("../api/desktop")>("../api/desktop");

  return {
    ...actual,
    fetchBootstrapState: vi.fn(),
    addNpcPlayers: vi.fn(),
    listNpcProfiles: vi.fn(),
    clientClaimLobbySeat: vi.fn(),
    clientSetLobbyReadyState: vi.fn(),
    getClientSessionStatus: vi.fn(),
    getHostSessionStatus: vi.fn(),
    subscribeBootstrap: vi.fn(),
    getDebugState: vi.fn(),
    hostClaimLobbySeat: vi.fn(),
    leaveClientSession: vi.fn(),
    hostSetLobbyReadyState: vi.fn(),
    hostStartTournament: vi.fn(),
    joinHostSession: vi.fn(),
    launchAdditionalClientInstance: vi.fn(),
    onSessionUpdate: vi.fn().mockResolvedValue(() => {}),
    onTableUpdate: vi.fn().mockResolvedValue(() => {}),
    resolveHostLanAddress: vi.fn(),
    stopHostSession: vi.fn(),
    startHostSession: vi.fn(),
    validateJoinPayloadInput: vi.fn(),
    getTableView: vi.fn(),
    submitTableAction: vi.fn(),
  };
});

import { h } from "./appShellHarness";

describe("AppShell integration (hand history screen)", () => {
  beforeEach(() => {
    h.setupAppShellMocks();
  });

  it("hand history populates from the live table view after a hand completes", async () => {
    // The HandHistoryScreen calls getTableView("local") on mount. When the
    // live table view contains handHistory entries those entries are shown
    // directly (not the persisted fallback).
    h.mockedGetTableView.mockResolvedValue(
      createTableViewSnapshot({
        handHistory: [
          {
            handNumber: 5,
            summary: "Alice won 300 chip(s).",
            potTotal: 300,
            winningPlayers: ["Alice"],
            eliminatedPlayers: [],
            boardCards: [],
          },
          {
            handNumber: 4,
            summary: "Bob won 200 chip(s).",
            potTotal: 200,
            winningPlayers: ["Bob"],
            eliminatedPlayers: [],
            boardCards: [],
          },
        ],
      }),
    );

    h.renderAppShell("/history");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Hand History" }),
    ).toBeTruthy();
    expect(await screen.findByText(/alice won 300 chip\(s\)\./i)).toBeTruthy();
    expect(screen.getByText(/bob won 200 chip\(s\)\./i)).toBeTruthy();
  });

  it("hand history falls back to the persisted cache when no live session is available", async () => {
    // getTableView rejects (session offline) → screen falls back to the
    // localStorage snapshot written by a prior table session.
    h.mockedGetTableView.mockRejectedValue(new Error("session offline"));

    const bootstrap = createAppBootstrap();
    persistHandHistory(bootstrap.storageNamespace, [
      {
        handNumber: 3,
        summary: "Cached winner won 150 chip(s).",
        potTotal: 150,
        winningPlayers: ["Cached winner"],
        eliminatedPlayers: [],
        boardCards: [],
      },
    ]);

    h.renderAppShell("/history", bootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Hand History" }),
    ).toBeTruthy();
    expect(
      await screen.findByText(/cached winner won 150 chip\(s\)\./i),
    ).toBeTruthy();
    expect(
      screen.getByText(/saved on this device/i),
    ).toBeTruthy();
  });

  it("hand history shows no-hands placeholder when session is gone and cache is empty", async () => {
    // No live session, no persisted history → the placeholder text must be shown
    // so the user is not left with an empty white screen.
    h.mockedGetTableView.mockRejectedValue(new Error("no session"));

    const freshBootstrap = createAppBootstrap({
      storageNamespace: "desktop-poker:fresh-ns",
      instanceId: "fresh-ns",
      instanceLabel: "Fresh",
    });

    h.renderAppShell("/history", freshBootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Hand History" }),
    ).toBeTruthy();
    expect(
      await screen.findByText(/no settled hands yet/i),
    ).toBeTruthy();
  });
});
