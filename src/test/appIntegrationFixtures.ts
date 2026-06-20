import type {
  DesktopBootstrapState,
  JoinPayload,
  TableViewSnapshot,
} from "../api/desktop";
import { createBootstrap } from "./fixtures";

export const DEFAULT_SCREEN_CATALOG: DesktopBootstrapState["screens"] = [
  { id: "home", title: "Home", route: "/", surface: "primary" },
  { id: "host", title: "Host", route: "/host", surface: "primary" },
  { id: "join", title: "Join", route: "/join", surface: "primary" },
  { id: "lobby", title: "Lobby", route: "/lobby", surface: "secondary" },
  { id: "table", title: "Table", route: "/table", surface: "primary" },
  { id: "history", title: "History", route: "/history", surface: "secondary" },
  {
    id: "complete",
    title: "Complete",
    route: "/complete",
    surface: "secondary",
  },
];

export function createAppBootstrap(
  overrides: Partial<DesktopBootstrapState> = {},
) {
  return createBootstrap({
    screens: DEFAULT_SCREEN_CATALOG,
    ...overrides,
  });
}

export function createParsedJoinPayload(
  overrides: Partial<JoinPayload> = {},
): JoinPayload {
  return {
    payloadVersion: 1,
    hostAddress: "192.168.1.10",
    hostPort: 43818,
    tableId: "table-123",
    sessionEpoch: 42,
    hostSigningPublicKey: "pub-key",
    joinToken: "join-token",
    generatedAtMs: 99,
    tableName: "Friday Night",
    ...overrides,
  };
}

export function createTableViewSnapshot(
  overrides: Partial<TableViewSnapshot> = {},
): TableViewSnapshot {
  return {
    viewerMode: "local",
    sessionConnection: "normal",
    tournamentPhase: "running",
    tournamentName: "Desktop Sit 'n Go",
    tableName: "Main Table",
    tableId: "desktop-shell-table",
    phaseLabel: "Running",
    streetLabel: "Flop",
    blindLevelLabel: "Level 1 · 10 / 20",
    currentHandNumber: 9,
    boardCards: [
      {
        label: "Ace of hearts",
        compactLabel: "A♥",
        suitSymbol: "♥",
        tone: "red",
      },
      {
        label: "King of clubs",
        compactLabel: "K♣",
        suitSymbol: "♣",
        tone: "dark",
      },
      {
        label: "Ten of diamonds",
        compactLabel: "10♦",
        suitSymbol: "♦",
        tone: "red",
      },
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
          {
            label: "Ace of spades",
            compactLabel: "A♠",
            suitSymbol: "♠",
            tone: "dark",
          },
          {
            label: "Ace of diamonds",
            compactLabel: "A♦",
            suitSymbol: "♦",
            tone: "red",
          },
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
      {
        sequence: 17,
        kind: "public-event",
        message: "The flop was published to every seat and observer.",
      },
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
