import { MemoryRouter } from "react-router-dom";
import "../App.css";
import type {
  DebugInspectorState,
  DesktopBootstrapState,
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
    backendModules: [],
    screens: [
      { id: "home", title: "Home", route: "/", surface: "primary" },
      { id: "host", title: "Host", route: "/host", surface: "primary" },
      { id: "join", title: "Join", route: "/join", surface: "primary" },
      { id: "history", title: "History", route: "/history", surface: "support" },
      { id: "rules", title: "Help", route: "/rules", surface: "support" },
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

function installBrowserMocks(bootstrap: DesktopBootstrapState) {
  const joinPayload = createProbeJoinPayload();
  const tableView = createProbeTableView();
  const noopUnsubscribe = async () => () => {};

  window.__DESKTOP_POKER_BROWSER_MOCKS__ = {
    fetchBootstrapState: async () => bootstrap,
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
  installBrowserMocks(bootstrap);
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