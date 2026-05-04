import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getTableView } from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { createTableViewSnapshot } from "../test/appIntegrationFixtures";
import { TournamentCompleteScreen } from "./TournamentCompleteScreen";

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

describe("TournamentCompleteScreen", () => {
  beforeEach(() => {
    mockedGetTableView.mockReset();
  });

  it("shows final standings when the table snapshot is available", async () => {
    const bootstrap = createBootstrap();
    mockedGetTableView.mockResolvedValue(
      createTableViewSnapshot({
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

    renderWithProviders(<TournamentCompleteScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    expect(await screen.findByText(/maya wins/i)).toBeTruthy();
    expect(screen.getByText(/#1 Maya/)).toBeTruthy();
    expect(screen.getByText(/won the tournament/i)).toBeTruthy();
  });
});