import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type BackendModuleDescriptor = {
  name: string;
  responsibility: string;
};

export type ScreenDescriptor = {
  id: string;
  title: string;
  route: string;
  surface: string;
};

export type JoinPayload = {
  payloadVersion: number;
  hostAddress: string;
  hostPort: number;
  tableId: string;
  sessionEpoch: number;
  hostSigningPublicKey: string;
  joinToken: string;
  generatedAtMs: number;
  tableName: string | null;
};

export type DesktopBootstrapState = {
  appName: string;
  protocolVersion: number;
  defaultHostPort: number;
  frontendStack: string;
  serializationStrategy: string;
  framingStrategy: string;
  joinPayloadEncoding: string;
  runtimeTransport: string;
  cryptoStack: string[];
  instanceId: string;
  instanceLabel: string;
  storageNamespace: string;
  sessionIdentity: string;
  reconnectNamespace: string;
  profileDirectory: string;
  launchJoinPayload: string | null;
  parsedLaunchJoinPayload: JoinPayload | null;
  launchJoinPayloadError: string | null;
  debugToolsEnabled: boolean;
  backendModules: BackendModuleDescriptor[];
  screens: ScreenDescriptor[];
};

export type TableViewerMode = "local" | "observer";

export type DesktopTableActionKind =
  | "fold"
  | "checkOrCall"
  | "betOrRaise"
  | "allIn";

export type TableCardView = {
  label: string;
  compactLabel: string;
  suitSymbol: string;
  tone: "red" | "dark";
};

export type TableSeatView = {
  seatIndex: number;
  displayName: string;
  chipCount: number | null;
  statusLabel: string;
  markerLabel: string | null;
  contribution: number;
  isLocal: boolean;
  isActing: boolean;
  isObserver: boolean;
  isEliminated: boolean;
  isCompact: boolean;
  cardsHidden: boolean;
  holeCards: TableCardView[];
  detailLines: string[];
};

export type TableStandingView = {
  rank: number;
  displayName: string;
  chipCount: number | null;
  statusLabel: string;
  note: string | null;
  isLocal: boolean;
  isObserver: boolean;
};

export type TableHistoryEntryView = {
  handNumber: number;
  summary: string;
  potTotal: number;
  winningPlayers: string[];
  eliminatedPlayers: string[];
  boardCards: TableCardView[];
};

export type TableEventView = {
  sequence: number;
  kind: string;
  message: string;
};

export type TableActionTrayView = {
  ownerLabel: string;
  checkOrCallLabel: string;
  betOrRaiseLabel: string;
  callAmount: number;
  currentBet: number;
  potTotal: number;
  minRaiseTo: number | null;
  maxRaiseTo: number | null;
  deadlineEpochMs: number;
  legalActions: string[];
};

export type TableViewSnapshot = {
  viewerMode: TableViewerMode;
  tournamentName: string;
  tableName: string;
  tableId: string;
  phaseLabel: string;
  streetLabel: string;
  blindLevelLabel: string;
  currentHandNumber: number | null;
  boardCards: TableCardView[];
  potTotal: number;
  actionOwnerLabel: string;
  eliminationSummary: string;
  observerBanner: string | null;
  seats: TableSeatView[];
  standings: TableStandingView[];
  handHistory: TableHistoryEntryView[];
  eventFeed: TableEventView[];
  actionTray: TableActionTrayView | null;
};

export type DebugInspectorState = {
  protocolLog: TableEventView[];
  snapshotJson: string;
  currentSequence: number;
  currentHandNumber: number | null;
  actionWindowSummary: string | null;
  launchHint: string;
};

const BOOTSTRAP_EVENT = "desktop://bootstrap";

export function fetchBootstrapState() {
  return invoke<DesktopBootstrapState>("get_bootstrap_state");
}

export function subscribeBootstrap(
  onBootstrap: (bootstrap: DesktopBootstrapState) => void,
) {
  return listen<DesktopBootstrapState>(BOOTSTRAP_EVENT, (event) => {
    onBootstrap(event.payload);
  });
}

export function validateJoinPayloadInput(payload: string) {
  return invoke<JoinPayload>("validate_join_payload_input", { payload });
}

export function resolveHostLanAddress() {
  return invoke<string>("resolve_host_lan_address");
}

export function getTableView(viewerMode: TableViewerMode) {
  return invoke<TableViewSnapshot>("get_table_view", { viewerMode });
}

export function submitTableAction(
  viewerMode: TableViewerMode,
  actionKind: DesktopTableActionKind,
  raiseToAmount?: number,
) {
  return invoke<TableViewSnapshot>("submit_table_action", {
    viewerMode,
    actionKind,
    raiseToAmount: raiseToAmount ?? null,
  });
}

export function getDebugState(viewerMode: TableViewerMode) {
  return invoke<DebugInspectorState>("get_debug_state", { viewerMode });
}

export function launchAdditionalClientInstance(joinPayload?: string | null) {
  return invoke<string>("launch_additional_client_instance", {
    joinPayload: joinPayload ?? null,
  });
}
