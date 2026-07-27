import { screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getTableView } from "../api/desktop";
import { readPersistedHandHistory } from "../app/persistence";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { createTableViewSnapshot } from "../test/appIntegrationFixtures";
import { TournamentCompleteScreen } from "./TournamentCompleteScreen";

vi.mock("../api/desktop", async () => {
  const actual =
    await vi.importActual<typeof import("../api/desktop")>("../api/desktop");

  return {
    ...actual,
    getTableView: vi.fn(),
  };
});

const mockedGetTableView = vi.mocked(getTableView);

describe("TournamentCompleteScreen", () => {
  beforeEach(() => {
    mockedGetTableView.mockReset();
    localStorage.clear();
  });

  it("shows final standings when the table snapshot is available", async () => {
    const bootstrap = createBootstrap();
    mockedGetTableView.mockResolvedValue(
      createTableViewSnapshot({
        tournamentPhase: "complete",
        phaseLabel: "Complete",
        standings: [
          {
            rank: 1,
            displayName: "Maya",
            chipCount: 1680,
            statusLabel: "Winner",
            note: "Won the tournament",
            isLocal: false,
            isObserver: false,
          },
          {
            rank: 2,
            displayName: "You",
            chipCount: 0,
            statusLabel: "Eliminated",
            note: "Finished 2nd",
            isLocal: true,
            isObserver: false,
          },
        ],
      }),
    );

    renderWithProviders(<TournamentCompleteScreen />, {
      bootstrap,
    });

    expect(await screen.findByText(/maya wins/i)).toBeTruthy();
    expect(screen.getByText(/#1 Maya/)).toBeTruthy();
    expect(screen.getByText(/won the tournament/i)).toBeTruthy();
  });

  it("shows player count and hands played stats on the complete screen (I3)", async () => {
    const bootstrap = createBootstrap();
    mockedGetTableView.mockResolvedValue(
      createTableViewSnapshot({
        tournamentPhase: "complete",
        phaseLabel: "Complete",
        currentHandNumber: 12,
        standings: [
          {
            rank: 1,
            displayName: "Maya",
            chipCount: 3000,
            statusLabel: "Winner",
            note: null,
            isLocal: false,
            isObserver: false,
          },
          {
            rank: 2,
            displayName: "You",
            chipCount: 0,
            statusLabel: "Eliminated",
            note: null,
            isLocal: true,
            isObserver: true,
          },
          {
            rank: 3,
            displayName: "Host",
            chipCount: 0,
            statusLabel: "Eliminated",
            note: null,
            isLocal: false,
            isObserver: true,
          },
        ],
      }),
    );

    renderWithProviders(<TournamentCompleteScreen />, { bootstrap });

    expect(await screen.findByText(/3 players/i)).toBeTruthy();
    expect(screen.getByText(/12 hands played/i)).toBeTruthy();
  });

  it("renders without crashing when standings array is empty (P0.3)", async () => {
    const bootstrap = createBootstrap();
    mockedGetTableView.mockResolvedValue(
      createTableViewSnapshot({
        tournamentPhase: "complete",
        phaseLabel: "Complete",
        standings: [],
      }),
    );

    renderWithProviders(<TournamentCompleteScreen />, { bootstrap });

    expect(await screen.findByText(/no result available/i)).toBeTruthy();
    expect(screen.queryByText(/undefined/i)).toBeNull();
  });

  it("shows statusLabel not chip count for observer standings entries (B1)", async () => {
    const bootstrap = createBootstrap();
    mockedGetTableView.mockResolvedValue(
      createTableViewSnapshot({
        tournamentPhase: "complete",
        phaseLabel: "Complete",
        standings: [
          {
            rank: 1,
            displayName: "Maya",
            chipCount: 3000,
            statusLabel: "Winner",
            note: null,
            isLocal: false,
            isObserver: false,
          },
          {
            rank: 2,
            displayName: "Riley",
            chipCount: 0,
            statusLabel: "Eliminated observer",
            note: null,
            isLocal: false,
            isObserver: true,
          },
        ],
      }),
    );

    renderWithProviders(<TournamentCompleteScreen />, { bootstrap });

    expect(await screen.findByText("Eliminated observer")).toBeTruthy();
    expect(screen.getByText("3000 chips")).toBeTruthy();
    const rileyHeading = screen.getByText(/#2 Riley/);
    const rileyRow = rileyHeading.closest("article");
    expect(rileyRow?.textContent).toContain("Eliminated observer");
    expect(rileyRow?.textContent).not.toMatch(/\d+ chips/);
  });

  it("persists the authoritative final hand history for restart recovery", async () => {
    const bootstrap = createBootstrap({
      storageNamespace: "desktop-poker:completion-persistence-test",
    });
    const handHistory = [
      {
        handNumber: 3,
        summary: "Maya won 3000 chip(s).",
        potTotal: 3000,
        winningPlayers: ["Maya"],
        eliminatedPlayers: ["You"],
        boardCards: [],
      },
    ];
    mockedGetTableView.mockResolvedValue(
      createTableViewSnapshot({
        tournamentPhase: "complete",
        phaseLabel: "Complete",
        currentHandNumber: 3,
        handHistory,
        standings: [
          {
            rank: 1,
            displayName: "Maya",
            chipCount: 3000,
            statusLabel: "Winner",
            note: "Won the tournament",
            isLocal: false,
            isObserver: false,
          },
        ],
      }),
    );

    renderWithProviders(<TournamentCompleteScreen />, { bootstrap });

    expect(await screen.findByText(handHistory[0].summary)).toBeTruthy();
    await waitFor(() => {
      expect(
        readPersistedHandHistory(bootstrap.storageNamespace)?.entries,
      ).toEqual(handHistory);
    });
    expect(screen.getByText("1 hand summaries saved.")).toBeTruthy();
  });

  it("retries a stale completion snapshot until the current hand arrives", async () => {
    const bootstrap = createBootstrap({
      storageNamespace: "desktop-poker:completion-retry-test",
    });
    const firstThreeHands = [
      {
        handNumber: 3,
        summary: "Hand three",
        potTotal: 300,
        winningPlayers: ["Maya"],
        eliminatedPlayers: [],
        boardCards: [],
      },
      {
        handNumber: 2,
        summary: "Hand two",
        potTotal: 200,
        winningPlayers: ["You"],
        eliminatedPlayers: [],
        boardCards: [],
      },
      {
        handNumber: 1,
        summary: "Hand one",
        potTotal: 100,
        winningPlayers: ["Maya"],
        eliminatedPlayers: [],
        boardCards: [],
      },
    ];
    const finalHand = {
      handNumber: 4,
      summary: "Maya won the final hand.",
      potTotal: 3000,
      winningPlayers: ["Maya"],
      eliminatedPlayers: ["You"],
      boardCards: [],
    };
    const standings = [
      {
        rank: 1,
        displayName: "Maya",
        chipCount: 3000,
        statusLabel: "Winner",
        note: "Won the tournament",
        isLocal: false,
        isObserver: false,
      },
    ];

    mockedGetTableView
      .mockResolvedValueOnce(
        createTableViewSnapshot({
          tournamentPhase: "complete",
          phaseLabel: "Complete",
          currentHandNumber: 4,
          handHistory: firstThreeHands,
          standings,
        }),
      )
      .mockResolvedValue(
        createTableViewSnapshot({
          tournamentPhase: "complete",
          phaseLabel: "Complete",
          currentHandNumber: 4,
          handHistory: [finalHand, ...firstThreeHands],
          standings,
        }),
      );

    renderWithProviders(<TournamentCompleteScreen />, { bootstrap });

    expect(await screen.findByText(finalHand.summary)).toBeTruthy();
    await waitFor(() => expect(mockedGetTableView).toHaveBeenCalledTimes(2));
    expect(
      readPersistedHandHistory(bootstrap.storageNamespace)?.entries,
    ).toEqual([finalHand, ...firstThreeHands]);
    expect(screen.getByText("4 hand summaries saved.")).toBeTruthy();
  });
});
