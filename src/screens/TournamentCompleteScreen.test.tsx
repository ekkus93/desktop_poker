import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getTableView } from "../api/desktop";
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

    // Riley's standings entry should show statusLabel, not a chip count
    expect(await screen.findByText("Eliminated observer")).toBeTruthy();
    // Maya (non-observer, 3000 chips) should show chip count
    expect(screen.getByText("3000 chips")).toBeTruthy();
    // Riley (observer) should NOT show "0 chips" — the field-hint for Riley's row
    // should contain "Eliminated observer", not a chip number
    const rileyHeading = screen.getByText(/#2 Riley/);
    const rileyRow = rileyHeading.closest("article");
    expect(rileyRow?.textContent).toContain("Eliminated observer");
    expect(rileyRow?.textContent).not.toMatch(/\d+ chips/);
  });
});
