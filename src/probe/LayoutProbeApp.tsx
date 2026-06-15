import { MemoryRouter } from "react-router-dom";
import "../App.css";
import type {
  DebugInspectorState,
  DesktopBootstrapState,
  HostSessionStatus,
  JoinPayload,
  TableViewSnapshot,
} from "../api/desktop";
import { DesktopShellProvider } from "../app/DesktopShellProvider";
import { persistHandHistory } from "../app/persistence";
import { storageKey, type HostDraft } from "../app/shell";
import { DeviceSettingsScreen } from "../screens/DeviceSettingsScreen";
import { HandHistoryScreen } from "../screens/HandHistoryScreen";
import { HomeScreen } from "../screens/HomeScreen";
import { HostTournamentSetupScreen } from "../screens/HostTournamentSetupScreen";
import { JoinTournamentScreen } from "../screens/JoinTournamentScreen";
import { MainTableScreen } from "../screens/MainTableScreen";
import { RulesHelpScreen } from "../screens/RulesHelpScreen";
import { TournamentCompleteScreen } from "../screens/TournamentCompleteScreen";
import { TournamentLobbyScreen } from "../screens/TournamentLobbyScreen";

type ProbeSurface =
  | "home"
  | "home-empty"
  | "host"
  | "join"
  | "lobby"
  | "table"
  | "table-dense"
  | "history"
  | "help"
  | "settings"
  | "complete";

function createProbeBootstrap(): DesktopBootstrapState {
  return {
    appName: "Desktop Poker",
    protocolVersion: 1,
    defaultHostPort: 43818,
    frontendStack: "React + TypeScript",
    serializationStrategy: "serde + canonical JSON bytes",
    framingStrategy: "length-prefixed JSON envelopes",
    joinPayloadEncoding: "pkr1_ compact join payload",
    runtimeTransport: "raw TCP over LAN",
    cryptoStack: ["ed25519-dalek", "x25519-dalek", "chacha20poly1305"],
    instanceId: "layout-probe",
    instanceLabel: "layout-probe",
    storageNamespace: "desktop-poker:layout-probe",
    sessionIdentity: "desktop-session:layout-probe",
    reconnectNamespace: "desktop-reconnect:layout-probe",
    profileDirectory: "/tmp/desktop-poker/layout-probe",
    launchJoinPayload: null,
    parsedLaunchJoinPayload: null,
    launchJoinPayloadError: null,
    debugToolsEnabled: false,
    llmApiKeyConfigured: false,
    llmProviderType: null,
    backendModules: [],
    screens: [
      { id: "home", title: "Home", route: "/", surface: "primary" },
      { id: "host", title: "Host", route: "/host", surface: "primary" },
      { id: "join", title: "Join", route: "/join", surface: "primary" },
      { id: "lobby", title: "Lobby", route: "/lobby", surface: "secondary" },
      { id: "table", title: "Table", route: "/table", surface: "primary" },
      { id: "history", title: "History", route: "/history", surface: "support" },
      {
        id: "complete",
        title: "Complete",
        route: "/complete",
        surface: "secondary",
      },
      { id: "rules", title: "Help", route: "/rules", surface: "support" },
      { id: "settings", title: "Settings", route: "/settings", surface: "support" },
    ],
  };
}

function createProbeJoinPayload(): JoinPayload {
  return {
    payloadVersion: 1,
    hostAddress: "192.168.1.10",
    hostPort: 43818,
    tableId: "layout-probe-table",
    sessionEpoch: 1,
    hostSigningPublicKey: "probe-public-key",
    joinToken: "probe-join-token",
    generatedAtMs: 1,
    tableName: "Friday Night",
  };
}

function createProbeTableView(): TableViewSnapshot {
  return {
    viewerMode: "local",
    tournamentName: "Desktop Sit 'n Go layout-probe",
    tableName: "Main Table",
    tableId: "layout-probe-table",
    phaseLabel: "Running",
    streetLabel: "Flop",
    blindLevelLabel: "Level 1 · 10 / 20",
    currentHandNumber: 9,
    boardCards: [
      { label: "Ace of spades", compactLabel: "A♠", suitSymbol: "♠", tone: "dark" },
      { label: "King of hearts", compactLabel: "K♥", suitSymbol: "♥", tone: "red" },
      { label: "Ten of clubs", compactLabel: "10♣", suitSymbol: "♣", tone: "dark" },
    ],
    potTotal: 120,
    actionOwnerLabel: "You",
    eliminationSummary: "Everyone still has chips.",
    observerBanner: null,
    seats: [
      {
        seatIndex: 1,
        displayName: "You",
        chipCount: 1480,
        statusLabel: "Active",
        markerLabel: "Dealer",
        contribution: 20,
        isLocal: true,
        isActing: true,
        isObserver: false,
        isEliminated: false,
        isCompact: false,
        cardsHidden: false,
        holeCards: [
          { label: "Ace of spades", compactLabel: "A♠", suitSymbol: "♠", tone: "dark" },
          { label: "Queen of spades", compactLabel: "Q♠", suitSymbol: "♠", tone: "dark" },
        ],
        detailLines: ["Probe local player"],
      },
      {
        seatIndex: 2,
        displayName: "Maya",
        chipCount: 1520,
        statusLabel: "Active",
        markerLabel: null,
        contribution: 20,
        isLocal: false,
        isActing: false,
        isObserver: false,
        isEliminated: false,
        isCompact: false,
        cardsHidden: true,
        holeCards: [],
        detailLines: ["Probe remote player"],
      },
    ],
    standings: [
      {
        rank: 1,
        displayName: "Maya",
        chipCount: 1520,
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
    ],
    handHistory: [
      {
        handNumber: 8,
        summary: "Host won 210 chip(s).",
        potTotal: 210,
        winningPlayers: ["Host"],
        eliminatedPlayers: [],
        boardCards: [],
      },
    ],
    eventFeed: [
      { sequence: 18, kind: "public-event", message: "The flop was published to every seat and observer." },
    ],
    actionTray: {
      ownerLabel: "You",
      checkOrCallLabel: "Check",
      betOrRaiseLabel: "Bet / Raise",
      callAmount: 20,
      currentBet: 20,
      potTotal: 120,
      minRaiseTo: 60,
      maxRaiseTo: 200,
      deadlineEpochMs: Date.now() + 60_000,
      legalActions: ["fold", "checkOrCall", "betOrRaise", "allIn"],
    },
  };
}

function createDenseProbeTableView(): TableViewSnapshot {
  const seats = Array.from({ length: 10 }, (_, index) => {
    const seatIndex = index + 1;
    const isLocal = seatIndex === 1;
    const compactCards = isLocal
      ? [
          { label: "Ace of spades", compactLabel: "A♠", suitSymbol: "♠", tone: "dark" as const },
          { label: "Queen of spades", compactLabel: "Q♠", suitSymbol: "♠", tone: "dark" as const },
        ]
      : [];

    return {
      seatIndex,
      displayName: isLocal ? "You" : `Seat ${seatIndex}`,
      chipCount: 1200 + (10 - seatIndex) * 25,
      statusLabel: seatIndex === 4 ? "All-in" : "Active",
      markerLabel: seatIndex === 1 ? "Dealer" : seatIndex === 2 ? "Small blind" : seatIndex === 3 ? "Big blind" : null,
      contribution: seatIndex <= 4 ? 20 * seatIndex : 0,
      isLocal,
      isActing: isLocal,
      isObserver: false,
      isEliminated: false,
      isCompact: !isLocal,
      cardsHidden: !isLocal,
      holeCards: compactCards,
      detailLines: [isLocal ? "Probe local player" : `Remote player ${seatIndex}`],
    };
  });

  return {
    viewerMode: "local",
    tournamentName: "Desktop Sit 'n Go layout-probe",
    tableName: "Main Table",
    tableId: "layout-probe-table-dense",
    phaseLabel: "Running",
    streetLabel: "Turn",
    blindLevelLabel: "Level 3 · 50 / 100",
    currentHandNumber: 18,
    boardCards: [
      { label: "Ace of hearts", compactLabel: "A♥", suitSymbol: "♥", tone: "red" },
      { label: "King of clubs", compactLabel: "K♣", suitSymbol: "♣", tone: "dark" },
      { label: "Ten of diamonds", compactLabel: "10♦", suitSymbol: "♦", tone: "red" },
      { label: "Five of spades", compactLabel: "5♠", suitSymbol: "♠", tone: "dark" },
    ],
    potTotal: 860,
    actionOwnerLabel: "You",
    eliminationSummary: "Ten seats active. One player is all-in.",
    observerBanner: null,
    seats,
    standings: seats.map((seat, index) => ({
      rank: index + 1,
      displayName: seat.displayName,
      chipCount: seat.chipCount,
      statusLabel: seat.statusLabel,
      note: null,
      isLocal: seat.isLocal,
      isObserver: false,
    })),
    handHistory: [
      {
        handNumber: 17,
        summary: "You won 860 chip(s).",
        potTotal: 860,
        winningPlayers: ["You"],
        eliminatedPlayers: [],
        boardCards: [],
      },
    ],
    eventFeed: [
      { sequence: 38, kind: "public-event", message: "The turn was published to every seat and observer." },
      { sequence: 39, kind: "public-event", message: "Seat 4 is all-in for 315 chips." },
    ],
    actionTray: {
      ownerLabel: "You",
      checkOrCallLabel: "Call 100",
      betOrRaiseLabel: "Raise",
      callAmount: 100,
      currentBet: 100,
      potTotal: 860,
      minRaiseTo: 200,
      maxRaiseTo: 600,
      deadlineEpochMs: Date.now() + 60_000,
      legalActions: ["fold", "checkOrCall", "betOrRaise", "allIn"],
    },
  };
}

function installBrowserMocks(bootstrap: DesktopBootstrapState, surface: ProbeSurface) {
  const joinPayload = createProbeJoinPayload();
  const tableView = surface === "table-dense" ? createDenseProbeTableView() : createProbeTableView();
  const hostSession: HostSessionStatus | null = surface === "lobby"
    ? {
        tournamentName: "Friday Finals",
        tableName: "Main Table",
        tableId: "layout-probe-table",
        sessionEpoch: 1,
        advertisedHost: "192.168.1.10",
        hostPort: 43818,
        invite: "pkr1_layout_probe_invite",
        phase: "waitingForPlayers",
        activeSeatCount: 2,
        openSeatCount: 8,
        participants: [
          {
            playerId: "local-player",
            displayName: "Player layout-probe",
            seatIndex: 0,
            isHost: true,
            isReady: false,
            connectionState: "connected",
            participantState: "seated",
          },
          {
            playerId: "player-b",
            displayName: "Maya",
            seatIndex: 1,
            isHost: false,
            isReady: false,
            connectionState: "connected",
            participantState: "seated",
          },
        ],
      }
    : null;
  const noopUnsubscribe = async () => () => {};

  window.__DESKTOP_POKER_BROWSER_MOCKS__ = {
    fetchBootstrapState: async () => bootstrap,
    getClientSessionStatus: async () => null,
    getHostSessionStatus: async () => hostSession,
    subscribeBootstrap: noopUnsubscribe,
    resolveHostLanAddress: async () => "192.168.1.10",
    validateJoinPayloadInput: async () => joinPayload,
    getTableView: async () => tableView,
    submitTableAction: async () => ({
      ...tableView,
      actionTray: null,
      actionOwnerLabel: "Maya",
    }),
    getDebugState: async () => ({
      protocolLog: [],
      snapshotJson: "{}",
      currentSequence: 1,
      currentHandNumber: tableView.currentHandNumber,
      actionWindowSummary: null,
      launchHint: "layout-probe",
      npcTiltLevels: {},
    } satisfies DebugInspectorState),
    launchAdditionalClientInstance: async () => "layout-probe-client",
  };
}

function seedProbeShellState(
  bootstrap: DesktopBootstrapState,
  hostDraftOverrides?: Partial<HostDraft>,
) {
  if (hostDraftOverrides) {
    localStorage.setItem(
      storageKey(bootstrap.storageNamespace, "host-draft"),
      JSON.stringify(hostDraftOverrides),
    );
  }

  localStorage.setItem(
    storageKey(bootstrap.storageNamespace, "recent-join-payloads"),
    JSON.stringify([createProbeJoinPayload().joinToken]),
  );
  persistHandHistory(bootstrap.storageNamespace, createProbeTableView().handHistory);
}

function renderProbeSurface(surface: ProbeSurface, bootstrap: DesktopBootstrapState) {
  switch (surface) {
    case "host":
      return <HostTournamentSetupScreen bootstrap={bootstrap} />;
    case "join":
      return <JoinTournamentScreen bootstrap={bootstrap} />;
    case "lobby":
      return <TournamentLobbyScreen bootstrap={bootstrap} />;
    case "table":
    case "table-dense":
      return <MainTableScreen bootstrap={bootstrap} />;
    case "history":
      return <HandHistoryScreen bootstrap={bootstrap} />;
    case "help":
      return <RulesHelpScreen />;
    case "settings":
      return <DeviceSettingsScreen />;
    case "complete":
      return <TournamentCompleteScreen />;
    case "home":
    case "home-empty":
    default:
      return <HomeScreen bootstrap={bootstrap} />;
  }
}

export function LayoutProbeApp({ surface }: { surface: string }) {
  const bootstrap = createProbeBootstrap();
  installBrowserMocks(bootstrap, surface as ProbeSurface);
  if (surface !== "home-empty") {
    seedProbeShellState(
      bootstrap,
      surface === "lobby"
        ? { tournamentName: "Friday Finals", maxPlayers: 10 }
        : undefined,
    );
  }

  return (
    <MemoryRouter initialEntries={["/"]}>
      <DesktopShellProvider bootstrap={bootstrap}>
        {renderProbeSurface(surface as ProbeSurface, bootstrap)}
      </DesktopShellProvider>
    </MemoryRouter>
  );
}
