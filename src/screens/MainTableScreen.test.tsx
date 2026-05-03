import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getTableView,
  submitTableAction,
  type DesktopTableActionKind,
  type TableViewSnapshot,
  type TableViewerMode,
} from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { MainTableScreen } from "./MainTableScreen";

vi.mock("../api/desktop", async () => {
  const actual = await vi.importActual<typeof import("../api/desktop")>(
    "../api/desktop",
  );

  return {
    ...actual,
    getTableView: vi.fn(),
    submitTableAction: vi.fn(),
  };
});

const mockedGetTableView = vi.mocked(getTableView);
const mockedSubmitTableAction = vi.mocked(submitTableAction);

function createTableView(
  overrides: Partial<TableViewSnapshot> = {},
): TableViewSnapshot {
  return {
    viewerMode: "local",
    tournamentName: "Desktop Sit 'n Go",
    tableName: "Main Table",
    tableId: "desktop-shell-table",
    phaseLabel: "Running",
    streetLabel: "Flop",
    blindLevelLabel: "Level 1 · 10 / 20",
    currentHandNumber: 9,
    boardCards: [
      { label: "Ace of hearts", compactLabel: "A♥", suitSymbol: "♥", tone: "red" },
      { label: "King of clubs", compactLabel: "K♣", suitSymbol: "♣", tone: "dark" },
      { label: "Ten of diamonds", compactLabel: "10♦", suitSymbol: "♦", tone: "red" },
    ],
    potTotal: 120,
    actionOwnerLabel: "You",
    eliminationSummary:
      "Riley busted on hand 8 and now remains at the table as a public-only observer.",
    observerBanner: null,
    seats: [
      {
        seatIndex: 1,
        displayName: "Host",
        chipCount: 1480,
        statusLabel: "Active",
        markerLabel: "Dealer",
        contribution: 20,
        isLocal: false,
        isActing: false,
        isObserver: false,
        isEliminated: false,
        isCompact: true,
        cardsHidden: true,
        holeCards: [],
        detailLines: ["Connection: Connected"],
      },
      {
        seatIndex: 2,
        displayName: "You",
        chipCount: 1520,
        statusLabel: "Active",
        markerLabel: null,
        contribution: 40,
        isLocal: true,
        isActing: true,
        isObserver: false,
        isEliminated: false,
        isCompact: false,
        cardsHidden: false,
        holeCards: [
          { label: "Ace of spades", compactLabel: "A♠", suitSymbol: "♠", tone: "dark" },
          { label: "Ace of diamonds", compactLabel: "A♦", suitSymbol: "♦", tone: "red" },
        ],
        detailLines: ["Connection: Connected"],
      },
      {
        seatIndex: 3,
        displayName: "Maya",
        chipCount: 1500,
        statusLabel: "Active",
        markerLabel: "Big blind",
        contribution: 60,
        isLocal: false,
        isActing: false,
        isObserver: false,
        isEliminated: false,
        isCompact: true,
        cardsHidden: true,
        holeCards: [],
        detailLines: ["Connection: Connected"],
      },
      {
        seatIndex: 6,
        displayName: "Riley",
        chipCount: 0,
        statusLabel: "Eliminated observer",
        markerLabel: "Observer",
        contribution: 0,
        isLocal: false,
        isActing: false,
        isObserver: true,
        isEliminated: true,
        isCompact: true,
        cardsHidden: true,
        holeCards: [],
        detailLines: ["Busted on hand 8 after finishing 4th."],
      },
    ],
    standings: [
      {
        rank: 1,
        displayName: "You",
        chipCount: 1520,
        statusLabel: "Active",
        note: null,
        isLocal: true,
        isObserver: false,
      },
      {
        rank: 2,
        displayName: "Maya",
        chipCount: 1500,
        statusLabel: "Active",
        note: null,
        isLocal: false,
        isObserver: false,
      },
      {
        rank: 3,
        displayName: "Host",
        chipCount: 1480,
        statusLabel: "Active",
        note: "Dealer",
        isLocal: false,
        isObserver: false,
      },
      {
        rank: 4,
        displayName: "Riley",
        chipCount: 0,
        statusLabel: "Eliminated observer",
        note: "Busted on hand 8",
        isLocal: false,
        isObserver: true,
      },
    ],
    handHistory: [
      {
        handNumber: 8,
        summary: "Host won 210 chip(s).",
        potTotal: 210,
        winningPlayers: ["Host"],
        eliminatedPlayers: ["Riley"],
        boardCards: [],
      },
    ],
    eventFeed: [
      { sequence: 17, kind: "public-event", message: "The flop was published to every seat and observer." },
    ],
    actionTray: {
      ownerLabel: "You",
      checkOrCallLabel: "Check",
      betOrRaiseLabel: "Bet / Raise",
      callAmount: 0,
      currentBet: 0,
      potTotal: 120,
      minRaiseTo: 60,
      maxRaiseTo: 1520,
      deadlineEpochMs: 123_456,
      legalActions: ["Fold", "Check", "Bet", "All-in"],
    },
    ...overrides,
  };
}

describe("MainTableScreen", () => {
  beforeEach(() => {
    mockedGetTableView.mockReset();
    mockedSubmitTableAction.mockReset();
  });

  it("enables the action tray only for the acting local player and blocks observer mode", async () => {
    const localView = createTableView();
    const observerView = createTableView({
      viewerMode: "observer",
      observerBanner:
        "Observer mode uses the public projector only: no private hole cards and no actions.",
      actionTray: null,
      actionOwnerLabel: "Maya",
      seats: createTableView().seats.map((seat) =>
        seat.isLocal ? { ...seat, cardsHidden: true } : seat,
      ),
    });

    mockedGetTableView.mockImplementation((viewerMode: TableViewerMode) =>
      Promise.resolve(viewerMode === "observer" ? observerView : localView),
    );

    const bootstrap = createBootstrap();
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, { bootstrap });

    expect(
      (await screen.findByRole("button", { name: "Fold" })) as HTMLButtonElement,
    ).toHaveProperty("disabled", false);
    expect(
      screen.getByRole("button", { name: "Check" }) as HTMLButtonElement,
    ).toHaveProperty("disabled", false);
    expect(screen.getByText("A♠")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Observer view" }));

    expect(
      await screen.findByText(/observer mode uses the public projector only/i),
    ).toBeTruthy();
    await waitFor(() => {
      expect(screen.queryByRole("button", { name: "Fold" })).toBeNull();
    });
    expect(
      screen.getByText(/observer mode keeps the table public-only/i),
    ).toBeTruthy();
  });

  it("reflects public events, history, and standings after settlement", async () => {
    const initialView = createTableView();
    const settledView = createTableView({
      streetLabel: "Turn",
      boardCards: [
        ...createTableView().boardCards,
        { label: "Jack of spades", compactLabel: "J♠", suitSymbol: "♠", tone: "dark" },
      ],
      potTotal: 240,
      actionOwnerLabel: "Waiting for settlement",
      standings: [
        {
          rank: 1,
          displayName: "Maya",
          chipCount: 1680,
          statusLabel: "Active",
          note: null,
          isLocal: false,
          isObserver: false,
        },
        {
          rank: 2,
          displayName: "You",
          chipCount: 1480,
          statusLabel: "Active",
          note: null,
          isLocal: true,
          isObserver: false,
        },
        ...createTableView().standings.slice(2),
      ],
      handHistory: [
        {
          handNumber: 9,
          summary: "Maya won 240 chip(s).",
          potTotal: 240,
          winningPlayers: ["Maya"],
          eliminatedPlayers: [],
          boardCards: [],
        },
        ...createTableView().handHistory,
      ],
      eventFeed: [
        { sequence: 18, kind: "public-event", message: "The turn was published to every seat and observer." },
        { sequence: 19, kind: "settlement", message: "Hand 9 settled: Maya collected the pot." },
      ],
      actionTray: null,
    });

    mockedGetTableView.mockResolvedValue(initialView);
    mockedSubmitTableAction.mockImplementation(
      async (
        viewerMode: TableViewerMode,
        actionKind: DesktopTableActionKind,
      ) => {
        expect(viewerMode).toBe("local");
        expect(actionKind).toBe("checkOrCall");
        return settledView;
      },
    );

    const bootstrap = createBootstrap();
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, { bootstrap });

    fireEvent.click(await screen.findByRole("button", { name: "Check" }));

    expect(
      await screen.findByText(/the turn was published to every seat and observer/i),
    ).toBeTruthy();
    expect(screen.getByText("J♠")).toBeTruthy();
    expect(screen.getByText(/maya won 240 chip\(s\)/i)).toBeTruthy();
    expect(screen.getByText(/#1 Maya/)).toBeTruthy();
    expect(
      screen.getByText(/waiting for the next local action window/i),
    ).toBeTruthy();
  });

  it("requires confirmation for raise and all-in actions", async () => {
    const initialView = createTableView();
    mockedGetTableView.mockResolvedValue(initialView);
    mockedSubmitTableAction.mockResolvedValue(
      createTableView({ actionTray: null, actionOwnerLabel: "Maya" }),
    );

    const bootstrap = createBootstrap();
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, { bootstrap });

    fireEvent.click(await screen.findByRole("button", { name: /bet \/ raise/i }));
    expect(await screen.findByText(/confirm raise/i)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(mockedSubmitTableAction).toHaveBeenCalledWith(
        "local",
        "betOrRaise",
        60,
      );
    });

    mockedSubmitTableAction.mockClear();
    mockedGetTableView.mockResolvedValue(initialView);
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, { bootstrap });

    fireEvent.click(await screen.findByRole("button", { name: "All-in" }));
    expect(await screen.findByText(/confirm all-in/i)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(mockedSubmitTableAction).toHaveBeenCalledWith(
        "local",
        "allIn",
        undefined,
      );
    });
  });
});
