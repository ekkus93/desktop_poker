import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type DesktopBrowserMocks = {
  fetchBootstrapState?: () => Promise<DesktopBootstrapState>;
  subscribeBootstrap?: (
    onBootstrap: (bootstrap: DesktopBootstrapState) => void,
  ) => Promise<() => void>;
  onSessionUpdate?: (callback: () => void) => Promise<() => void>;
  onTableUpdate?: (callback: () => void) => Promise<() => void>;
  addNpcPlayers?: (request: AddNpcPlayersRequest) => Promise<HostSessionStatus>;
  startHostSession?: (
    request: StartHostSessionRequest,
  ) => Promise<HostSessionStatus>;
  getHostSessionStatus?: () => Promise<HostSessionStatus | null>;
  stopHostSession?: () => Promise<void>;
  hostClaimLobbySeat?: (
    request: ClaimLobbySeatRequest,
  ) => Promise<HostSessionStatus>;
  hostSetLobbyReadyState?: (
    request: SetLobbyReadyStateRequest,
  ) => Promise<HostSessionStatus>;
  hostStartTournament?: () => Promise<HostSessionStatus>;
  joinHostSession?: (
    request: JoinHostSessionRequest,
  ) => Promise<ClientSessionStatus>;
  getClientSessionStatus?: () => Promise<ClientSessionStatus | null>;
  leaveClientSession?: () => Promise<void>;
  clientClaimLobbySeat?: (
    request: ClaimLobbySeatRequest,
  ) => Promise<ClientSessionStatus>;
  clientSetLobbyReadyState?: (
    request: SetLobbyReadyStateRequest,
  ) => Promise<ClientSessionStatus>;
  validateJoinPayloadInput?: (payload: string) => Promise<JoinPayload>;
  resolveHostLanAddress?: () => Promise<string>;
  getTableView?: (viewerMode: TableViewerMode) => Promise<TableViewSnapshot>;
  submitTableAction?: (
    viewerMode: TableViewerMode,
    actionKind: DesktopTableActionKind,
    raiseToAmount?: number,
  ) => Promise<TableViewSnapshot>;
  getDebugState?: (viewerMode: TableViewerMode) => Promise<DebugInspectorState>;
  launchAdditionalClientInstance?: (
    joinPayload?: string | null,
  ) => Promise<string>;
};

declare global {
  interface Window {
    __DESKTOP_POKER_BROWSER_MOCKS__?: DesktopBrowserMocks;
  }
}

function getBrowserMocks() {
  if (typeof window === "undefined") {
    return undefined;
  }

  return window.__DESKTOP_POKER_BROWSER_MOCKS__;
}

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

export type StartHostSessionRequest = {
  hostAddress: string;
  hostPort: number;
  tournamentName: string;
  maxPlayers: number;
  startingStack: number;
  blindPresetId: string;
  turnTimerSeconds: number;
  displayName: string;
};

export type HostSessionParticipantView = {
  playerId: string;
  displayName: string;
  seatIndex: number | null;
  isHost: boolean;
  isReady: boolean;
  connectionState: string;
  participantState: string;
};

export type HostSessionStatus = {
  tournamentName: string;
  tableName: string;
  tableId: string;
  sessionEpoch: number;
  advertisedHost: string;
  hostPort: number;
  invite: string;
  phase: string;
  activeSeatCount: number;
  openSeatCount: number;
  participants: HostSessionParticipantView[];
};

export type JoinHostSessionRequest = {
  joinPayload: string;
  displayName: string;
};

export type ClaimLobbySeatRequest = {
  seatIndex: number;
};

export type SetLobbyReadyStateRequest = {
  isReady: boolean;
};

export type ClientSessionStatus = {
  tournamentName: string;
  tableName: string;
  tableId: string;
  sessionEpoch: number;
  hostAddress: string;
  hostPort: number;
  localPlayerId: string;
  phase: string;
  activeSeatCount: number;
  openSeatCount: number;
  reconnecting: boolean;
  lastError: string | null;
  participants: HostSessionParticipantView[];
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
  llmApiKeyConfigured: boolean;
  llmProviderType: string | null;
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
  /** Maps NPC player_id → tilt level ("none", "mild", "full"). Empty when no host session. */
  npcTiltLevels: Record<string, string>;
};

const BOOTSTRAP_EVENT = "desktop://bootstrap";
const SESSION_UPDATE_EVENT = "desktop://session-update";
const TABLE_UPDATE_EVENT = "desktop://table-update";

export function fetchBootstrapState() {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.fetchBootstrapState) {
    return browserMocks.fetchBootstrapState();
  }

  return invoke<DesktopBootstrapState>("get_bootstrap_state");
}

export function subscribeBootstrap(
  onBootstrap: (bootstrap: DesktopBootstrapState) => void,
) {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.subscribeBootstrap) {
    return browserMocks.subscribeBootstrap(onBootstrap);
  }

  return listen<DesktopBootstrapState>(BOOTSTRAP_EVENT, (event) => {
    onBootstrap(event.payload);
  });
}

export function onSessionUpdate(callback: () => void) {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.onSessionUpdate) {
    return browserMocks.onSessionUpdate(callback);
  }

  return listen(SESSION_UPDATE_EVENT, () => {
    callback();
  });
}

export function onTableUpdate(callback: () => void) {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.onTableUpdate) {
    return browserMocks.onTableUpdate(callback);
  }

  return listen(TABLE_UPDATE_EVENT, () => {
    callback();
  });
}

export type NpcStyle = "aggressive" | "conservative";

export type NpcConfig = {
  displayName: string;
  style: NpcStyle;
  profileId?: string | null;
};

export type AddNpcPlayersRequest = {
  npcs: NpcConfig[];
};

export function addNpcPlayers(request: AddNpcPlayersRequest) {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.addNpcPlayers) {
    return browserMocks.addNpcPlayers(request);
  }

  return invoke<HostSessionStatus>("add_npc_players", { request });
}

export type NpcProfile = {
  id: string;
  name: string;
  style: string;
  skill: string;
  description: string;
  opponentTendencies: string | null;
  tiltBehaviour: string | null;
};

export function listNpcProfiles(): Promise<NpcProfile[]> {
  return invoke<NpcProfile[]>("list_npc_profiles");
}

export function getNpcProfile(id: string): Promise<NpcProfile> {
  return invoke<NpcProfile>("get_npc_profile", { id });
}

export function saveNpcProfile(
  id: string,
  content: string,
): Promise<NpcProfile> {
  return invoke<NpcProfile>("save_npc_profile", { id, content });
}

export function deleteNpcProfile(id: string): Promise<void> {
  return invoke<void>("delete_npc_profile", { id });
}

export function setLlmApiKey(key: string): Promise<void> {
  return invoke<void>("set_llm_api_key", { key });
}

export function clearLlmApiKey(): Promise<void> {
  return invoke<void>("clear_llm_api_key");
}

export type LlmProviderType = "anthropic" | "openAi" | "ollama" | "llamaServer";

export type LlmProviderConfig = {
  provider: LlmProviderType;
  apiKey: string | null;
  endpointUrl: string | null;
  model: string | null;
};

export function getLlmProviderConfig(): Promise<LlmProviderConfig | null> {
  return invoke<LlmProviderConfig | null>("get_llm_provider_config");
}

export function setLlmProviderConfig(config: LlmProviderConfig): Promise<void> {
  return invoke<void>("set_llm_provider_config", { config });
}

export function clearLlmProviderConfig(): Promise<void> {
  return invoke<void>("clear_llm_provider_config");
}

export function validateJoinPayloadInput(payload: string) {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.validateJoinPayloadInput) {
    return browserMocks.validateJoinPayloadInput(payload);
  }

  return invoke<JoinPayload>("validate_join_payload_input", { payload });
}

export function startHostSession(request: StartHostSessionRequest) {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.startHostSession) {
    return browserMocks.startHostSession(request);
  }

  return invoke<HostSessionStatus>("start_host_session", { request });
}

export function getHostSessionStatus() {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.getHostSessionStatus) {
    return browserMocks.getHostSessionStatus();
  }

  return invoke<HostSessionStatus | null>("get_host_session_status");
}

export function stopHostSession() {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.stopHostSession) {
    return browserMocks.stopHostSession();
  }

  return invoke<void>("stop_host_session");
}

export function hostClaimLobbySeat(request: ClaimLobbySeatRequest) {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.hostClaimLobbySeat) {
    return browserMocks.hostClaimLobbySeat(request);
  }

  return invoke<HostSessionStatus>("host_claim_lobby_seat", { request });
}

export function hostSetLobbyReadyState(request: SetLobbyReadyStateRequest) {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.hostSetLobbyReadyState) {
    return browserMocks.hostSetLobbyReadyState(request);
  }

  return invoke<HostSessionStatus>("host_set_lobby_ready_state", { request });
}

export function hostStartTournament() {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.hostStartTournament) {
    return browserMocks.hostStartTournament();
  }

  return invoke<HostSessionStatus>("host_start_tournament");
}

export function joinHostSession(request: JoinHostSessionRequest) {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.joinHostSession) {
    return browserMocks.joinHostSession(request);
  }

  return invoke<ClientSessionStatus>("join_host_session", { request });
}

export function getClientSessionStatus() {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.getClientSessionStatus) {
    return browserMocks.getClientSessionStatus();
  }

  return invoke<ClientSessionStatus | null>("get_client_session_status");
}

export function leaveClientSession() {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.leaveClientSession) {
    return browserMocks.leaveClientSession();
  }

  return invoke<void>("leave_client_session");
}

export function clientClaimLobbySeat(request: ClaimLobbySeatRequest) {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.clientClaimLobbySeat) {
    return browserMocks.clientClaimLobbySeat(request);
  }

  return invoke<ClientSessionStatus>("client_claim_lobby_seat", { request });
}

export function clientSetLobbyReadyState(request: SetLobbyReadyStateRequest) {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.clientSetLobbyReadyState) {
    return browserMocks.clientSetLobbyReadyState(request);
  }

  return invoke<ClientSessionStatus>("client_set_lobby_ready_state", {
    request,
  });
}

export function resolveHostLanAddress() {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.resolveHostLanAddress) {
    return browserMocks.resolveHostLanAddress();
  }

  return invoke<string>("resolve_host_lan_address");
}

export function getTableView(viewerMode: TableViewerMode) {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.getTableView) {
    return browserMocks.getTableView(viewerMode);
  }

  return invoke<TableViewSnapshot>("get_table_view", { viewerMode });
}

export function submitTableAction(
  viewerMode: TableViewerMode,
  actionKind: DesktopTableActionKind,
  raiseToAmount?: number,
) {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.submitTableAction) {
    return browserMocks.submitTableAction(
      viewerMode,
      actionKind,
      raiseToAmount,
    );
  }

  return invoke<TableViewSnapshot>("submit_table_action", {
    viewerMode,
    actionKind,
    raiseToAmount: raiseToAmount ?? null,
  });
}

export function getDebugState(viewerMode: TableViewerMode) {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.getDebugState) {
    return browserMocks.getDebugState(viewerMode);
  }

  return invoke<DebugInspectorState>("get_debug_state", { viewerMode });
}

export function launchAdditionalClientInstance(joinPayload?: string | null) {
  const browserMocks = getBrowserMocks();
  if (browserMocks?.launchAdditionalClientInstance) {
    return browserMocks.launchAdditionalClientInstance(joinPayload);
  }

  return invoke<string>("launch_additional_client_instance", {
    joinPayload: joinPayload ?? null,
  });
}
