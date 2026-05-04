import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getTableView, type TableViewSnapshot } from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import {
  createHistoryEntry,
  seedPersistedHandHistory,
} from "../test/persistenceFixtures";
import { HandHistoryScreen } from "./HandHistoryScreen";

vi.mock("../api/desktop", async () => {
  const actual = await vi.importActual<typeof import("../api/desktop")>(
    "../api/desktop",
  );

  return {
    ...actual,
    getTableView: vi.fn(),
  };
});

const mockedGetTableView = vi.mocked(getTableView);

function createHistorySnapshot(
  overrides: Partial<TableViewSnapshot> = {},
): TableViewSnapshot {
  return {
    viewerMode: "local",
    tournamentName: "Desktop Sit 'n Go",
    tableName: "Main Table",
    tableId: "desktop-shell-table",
    phaseLabel: "Running",
    streetLabel: "River",
    blindLevelLabel: "Level 1 · 10 / 20",
    currentHandNumber: 10,
    boardCards: [],
    potTotal: 240,
    actionOwnerLabel: "Waiting",
    eliminationSummary: "No eliminations yet.",
    observerBanner: null,
    seats: [],
    standings: [],
    handHistory: [
      createHistoryEntry({
        handNumber: 9,
        summary: "Maya won 240 chip(s).",
        potTotal: 240,
        winningPlayers: ["Maya"],
      }),
    ],
    eventFeed: [],
    actionTray: null,
    ...overrides,
  };
}

describe("HandHistoryScreen", () => {
  beforeEach(() => {
    mockedGetTableView.mockReset();
  });

  it("shows saved hand-history summaries when the live fetch fails", async () => {
    const bootstrap = createBootstrap();
    seedPersistedHandHistory(
      bootstrap.storageNamespace,
      createHistorySnapshot().handHistory,
    );
    mockedGetTableView.mockRejectedValue(new Error("offline"));

    renderWithProviders(<HandHistoryScreen bootstrap={bootstrap} />, { bootstrap });

    expect(await screen.findByText("offline")).toBeTruthy();
    expect(
      screen.getByText(/showing locally saved hand summaries for this device/i),
    ).toBeTruthy();
    expect(screen.getByText(/maya won 240 chip\(s\)/i)).toBeTruthy();
  });

  it("replaces cached hand history with live data when the fetch succeeds", async () => {
    const bootstrap = createBootstrap();
    seedPersistedHandHistory(bootstrap.storageNamespace, [
      createHistoryEntry({ handNumber: 2, summary: "Cached summary" }),
    ]);
    mockedGetTableView.mockResolvedValue(
      createHistorySnapshot({
        handHistory: [
          createHistoryEntry({ handNumber: 7, summary: "Live summary" }),
        ],
      }),
    );

    renderWithProviders(<HandHistoryScreen bootstrap={bootstrap} />, { bootstrap });

    expect(await screen.findByText(/live summary/i)).toBeTruthy();
    expect(screen.queryByText(/cached summary/i)).toBeNull();
    expect(
      screen.queryByText(/showing locally saved hand-history summaries/i),
    ).toBeNull();
  });

  it("shows the empty state when neither live nor cached history is available", async () => {
    const bootstrap = createBootstrap();
    mockedGetTableView.mockResolvedValue(
      createHistorySnapshot({
        handHistory: [],
        standings: [],
      }),
    );

    renderWithProviders(<HandHistoryScreen bootstrap={bootstrap} />, { bootstrap });

    expect(
      await screen.findByText(
        /no settled hands yet\. return to the table to keep the game moving\./i,
      ),
    ).toBeTruthy();
  });

  it("ignores malformed cached history and still renders safely on fetch failure", async () => {
    const bootstrap = createBootstrap();
    localStorage.setItem(
      `${bootstrap.storageNamespace}:hand-history-summaries`,
      JSON.stringify({
        updatedAtMs: 1234,
        entries: [{ handNumber: "bad-entry" }],
      }),
    );
    mockedGetTableView.mockRejectedValue(new Error("offline"));

    renderWithProviders(<HandHistoryScreen bootstrap={bootstrap} />, { bootstrap });

    expect(await screen.findByText("offline")).toBeTruthy();
    expect(
      screen.getByText(
        /no settled hands yet\. return to the table to keep the game moving\./i,
      ),
    ).toBeTruthy();
  });
});
