import { fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getTableView, type TableViewSnapshot } from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import {
  createHistoryEntry,
  seedPersistedHandHistory,
} from "../test/persistenceFixtures";
import { HandHistoryScreen } from "./HandHistoryScreen";

vi.mock("../api/desktop", async () => {
  const actual =
    await vi.importActual<typeof import("../api/desktop")>("../api/desktop");

  return {
    ...actual,
    getTableView: vi.fn(),
  };
});

const mockedGetTableView = vi.mocked(getTableView);
const tauriInvokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: tauriInvokeMock,
}));

function createHistorySnapshot(
  overrides: Partial<TableViewSnapshot> = {},
): TableViewSnapshot {
  return {
    viewerMode: "local",
    sessionConnection: "normal",
    tournamentPhase: "running",
    tournamentName: "Desktop Sit 'n Go",
    tableName: "Main Table",
    tableId: "desktop-shell-table",
    sessionEpoch: 42,
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

function createLongHistory(count: number) {
  return Array.from({ length: count }, (_, index) => {
    const handNumber = index + 1;
    return createHistoryEntry({
      handNumber,
      summary: `Player won hand ${handNumber}`,
      potTotal: handNumber * 10,
    });
  });
}

describe("HandHistoryScreen", () => {
  beforeEach(() => {
    mockedGetTableView.mockReset();
    tauriInvokeMock.mockReset();
    delete (window as typeof window & { __TAURI_INTERNALS__?: unknown })
      .__TAURI_INTERNALS__;
  });

  it("shows saved hand-history summaries when the live fetch fails", async () => {
    const bootstrap = createBootstrap();
    seedPersistedHandHistory(
      bootstrap.storageNamespace,
      createHistorySnapshot().handHistory,
    );
    mockedGetTableView.mockRejectedValue(new Error("offline"));

    renderWithProviders(<HandHistoryScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    expect(await screen.findByText("offline")).toBeTruthy();
    expect(await screen.findByText(/maya won 240 chip\(s\)/i)).toBeTruthy();
    expect(screen.getByText(/saved on this device/i)).toBeTruthy();
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

    renderWithProviders(<HandHistoryScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

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

    renderWithProviders(<HandHistoryScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    expect(await screen.findByText(/no settled hands yet\./i)).toBeTruthy();
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

    renderWithProviders(<HandHistoryScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    expect(await screen.findByText("offline")).toBeTruthy();
    expect(screen.getByText(/no settled hands yet\./i)).toBeTruthy();
  });

  it("keeps equal hand numbers from distinct saved tournaments selectable", async () => {
    const bootstrap = createBootstrap();
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    const persistedHistory = {
      schemaVersion: 2,
      updatedAtMs: 1234,
      entries: [
        {
          ...createHistoryEntry({
            handNumber: 1,
            summary: "Older tournament hand one",
          }),
          provenance: "live",
          tableId: "table-old",
          sessionEpoch: 100,
          tournamentName: "Older tournament",
        },
        {
          ...createHistoryEntry({
            handNumber: 1,
            summary: "Newer tournament hand one",
          }),
          provenance: "live",
          tableId: "table-new",
          sessionEpoch: 200,
          tournamentName: "Newer tournament",
        },
      ],
    };
    tauriInvokeMock.mockImplementation(async (command: string) =>
      command === "get_runtime_warnings" ? [] : persistedHistory,
    );
    mockedGetTableView.mockResolvedValue(
      createHistorySnapshot({
        tableId: "table-new",
        sessionEpoch: 200,
        tournamentName: "Newer tournament",
        handHistory: [],
      }),
    );

    renderWithProviders(<HandHistoryScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    expect(await screen.findByText("Newer tournament hand one")).toBeTruthy();
    expect(screen.queryByText("Older tournament hand one")).toBeNull();
    fireEvent.change(screen.getByLabelText("Tournament"), {
      target: { value: "live:table-old:100" },
    });
    expect(screen.getByText("Older tournament hand one")).toBeTruthy();
    expect(screen.queryByText("Newer tournament hand one")).toBeNull();
    expect(tauriInvokeMock).toHaveBeenCalledWith(
      "read_persisted_hand_history_v2",
    );
  });

  it.each([100, 101])(
    "shows the newest 100 of %i hands with accurate older paging",
    async (count) => {
      const bootstrap = createBootstrap();
      mockedGetTableView.mockResolvedValue(
        createHistorySnapshot({
          currentHandNumber: count + 1,
          handHistory: createLongHistory(count),
        }),
      );

      renderWithProviders(<HandHistoryScreen bootstrap={bootstrap} />, {
        bootstrap,
      });

      expect(
        await screen.findByText(
          `Showing 100 of ${count} settled hands, newest first in the selected tournament.`,
        ),
      ).toBeTruthy();
      expect(screen.getByText(`Player won hand ${count}`)).toBeTruthy();
      expect(screen.getByText(`Player won hand ${count - 99}`)).toBeTruthy();
      expect(screen.getAllByText(/^Player won hand \d+$/)).toHaveLength(100);

      if (count === 100) {
        expect(
          screen.queryByRole("button", { name: /older hand/i }),
        ).toBeNull();
      } else {
        expect(screen.queryByText("Player won hand 1")).toBeNull();
        fireEvent.click(
          screen.getByRole("button", { name: "Show 1 older hand" }),
        );
        expect(screen.getByText("Player won hand 1")).toBeTruthy();
        expect(screen.getAllByText(/^Player won hand \d+$/)).toHaveLength(101);
        expect(
          screen.queryByRole("button", { name: /older hand/i }),
        ).toBeNull();
      }
    },
  );

  it("bounds the initial render and exposes every older hand incrementally", async () => {
    const bootstrap = createBootstrap();
    mockedGetTableView.mockResolvedValue(
      createHistorySnapshot({
        currentHandNumber: 206,
        handHistory: createLongHistory(205),
      }),
    );

    renderWithProviders(<HandHistoryScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    expect(
      await screen.findByText(/showing 100 of 205 settled hands/i),
    ).toBeTruthy();
    expect(screen.queryByText("Player won hand 105")).toBeNull();
    expect(screen.getByText("Player won hand 106")).toBeTruthy();
    expect(screen.getByText("Player won hand 205")).toBeTruthy();
    expect(screen.getAllByText(/^Player won hand \d+$/)[0].textContent).toBe(
      "Player won hand 205",
    );

    fireEvent.click(
      screen.getByRole("button", { name: "Show 100 older hands" }),
    );

    expect(screen.getByText(/showing 200 of 205 settled hands/i)).toBeTruthy();
    expect(screen.queryByText("Player won hand 5")).toBeNull();
    expect(screen.getByText("Player won hand 6")).toBeTruthy();
    const visibleHandSummaries = screen.getAllByText(/^Player won hand \d+$/);
    expect(
      visibleHandSummaries[visibleHandSummaries.length - 1]?.textContent,
    ).toBe("Player won hand 6");

    fireEvent.click(screen.getByRole("button", { name: "Show 5 older hands" }));

    expect(screen.getByText(/showing 205 of 205 settled hands/i)).toBeTruthy();
    expect(screen.getByText("Player won hand 1")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /older hands/i })).toBeNull();
  });
});
