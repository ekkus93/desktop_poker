import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  clientClaimLobbySeat,
  clientSetLobbyReadyState,
  type ClaimLobbySeatRequest,
  fetchBootstrapState,
  getClientSessionStatus,
  hostClaimLobbySeat,
  getHostSessionStatus,
  getDebugState,
  getTableView,
  hostSetLobbyReadyState,
  hostStartTournament,
  joinHostSession,
  launchAdditionalClientInstance,
  leaveClientSession,
  resolveHostLanAddress,
  startHostSession,
  stopHostSession,
  submitTableAction,
  subscribeBootstrap,
  validateJoinPayloadInput,
  type DebugInspectorState,
  type DesktopBootstrapState,
  type ClientSessionStatus,
  type HostSessionStatus,
  type JoinHostSessionRequest,
  type JoinPayload,
  type SetLobbyReadyStateRequest,
  type StartHostSessionRequest,
  type TableViewSnapshot,
} from "./desktop";
import { createBootstrap } from "../test/fixtures";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const mockedInvoke = vi.mocked(invoke);
const mockedListen = vi.mocked(listen);

function setBrowserMocks(mocks?: Record<string, unknown>) {
  Object.defineProperty(window, "__DESKTOP_POKER_BROWSER_MOCKS__", {
    configurable: true,
    value: mocks,
    writable: true,
  });
}

function sampleTableView(): TableViewSnapshot {
  return {
    viewerMode: "local",
    tournamentName: "Friday Night",
    tableName: "Table 1",
    tableId: "table-1",
    phaseLabel: "Running",
    streetLabel: "Flop",
    blindLevelLabel: "10 / 20",
    currentHandNumber: 12,
    boardCards: [],
    potTotal: 120,
    actionOwnerLabel: "Alice",
    eliminationSummary: "",
    observerBanner: null,
    seats: [],
    standings: [],
    handHistory: [],
    eventFeed: [],
    actionTray: null,
  };
}

function sampleDebugState(): DebugInspectorState {
  return {
    protocolLog: [],
    snapshotJson: "{}",
    currentSequence: 7,
    currentHandNumber: 2,
    actionWindowSummary: null,
    launchHint: "hint",
  };
}

function sampleJoinPayload(): JoinPayload {
  return {
    payloadVersion: 1,
    hostAddress: "192.168.1.5",
    hostPort: 43818,
    tableId: "table-1",
    sessionEpoch: 9,
    hostSigningPublicKey: "pubkey",
    joinToken: "token",
    generatedAtMs: 1,
    tableName: "Friday Night",
  };
}

function sampleStartHostSessionRequest(): StartHostSessionRequest {
  return {
    hostAddress: "192.168.1.10",
    hostPort: 43818,
    tournamentName: "Friday Night",
    maxPlayers: 6,
    startingStack: 1500,
    blindPresetId: "normal",
    turnTimerSeconds: 30,
    displayName: "Host Alpha",
  };
}

function sampleHostSessionStatus(): HostSessionStatus {
  return {
    tournamentName: "Friday Night",
    tableName: "Main Table",
    tableId: "table-1",
    sessionEpoch: 42,
    advertisedHost: "192.168.1.10",
    hostPort: 43818,
    invite: "pkr1_live",
    phase: "waitingForPlayers",
    activeSeatCount: 1,
    openSeatCount: 5,
    participants: [
      {
        playerId: "local-player",
        displayName: "Host Alpha",
        seatIndex: 0,
        isHost: true,
        isReady: false,
        connectionState: "connected",
        participantState: "seated",
      },
    ],
  };
}

function sampleJoinHostSessionRequest(): JoinHostSessionRequest {
  return {
    joinPayload: "pkr1_live",
    displayName: "Client Bravo",
  };
}

function sampleClaimLobbySeatRequest(): ClaimLobbySeatRequest {
  return {
    seatIndex: 2,
  };
}

function sampleSetLobbyReadyStateRequest(): SetLobbyReadyStateRequest {
  return {
    isReady: true,
  };
}

function sampleClientSessionStatus(): ClientSessionStatus {
  return {
    tournamentName: "Friday Night",
    tableName: "Main Table",
    tableId: "table-1",
    sessionEpoch: 42,
    hostAddress: "192.168.1.10",
    hostPort: 43818,
    localPlayerId: "player-client-b",
    phase: "waitingForPlayers",
    activeSeatCount: 1,
    openSeatCount: 5,
    reconnecting: false,
    lastError: null,
    participants: sampleHostSessionStatus().participants,
  };
}

describe("desktop API bridge", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedListen.mockReset();
    setBrowserMocks(undefined);
  });

  describe("fetchBootstrapState", () => {
    it("prefers browser mocks when present", async () => {
      const bootstrap = createBootstrap({ instanceId: "browser" });
      const fetchBootstrapStateMock = vi.fn().mockResolvedValue(bootstrap);
      setBrowserMocks({ fetchBootstrapState: fetchBootstrapStateMock });

      await expect(fetchBootstrapState()).resolves.toEqual(bootstrap);
      expect(fetchBootstrapStateMock).toHaveBeenCalledTimes(1);
      expect(mockedInvoke).not.toHaveBeenCalled();
    });

    it("invokes the exact bootstrap command when browser mocks are absent", async () => {
      const bootstrap = createBootstrap({ instanceId: "tauri" });
      mockedInvoke.mockResolvedValue(bootstrap);

      await expect(fetchBootstrapState()).resolves.toEqual(bootstrap);
      expect(mockedInvoke).toHaveBeenCalledWith("get_bootstrap_state");
    });
  });

  describe("subscribeBootstrap", () => {
    it("prefers browser mocks when present", async () => {
      const unsubscribe = vi.fn();
      const subscribeBootstrapMock = vi.fn().mockResolvedValue(unsubscribe);
      const onBootstrap = vi.fn();
      setBrowserMocks({ subscribeBootstrap: subscribeBootstrapMock });

      await expect(subscribeBootstrap(onBootstrap)).resolves.toBe(unsubscribe);
      expect(subscribeBootstrapMock).toHaveBeenCalledWith(onBootstrap);
      expect(mockedListen).not.toHaveBeenCalled();
    });

    it("subscribes to the desktop bootstrap event and forwards payloads", async () => {
      const unsubscribe = vi.fn();
      const bootstrap = createBootstrap({ instanceId: "event-host" });
      const onBootstrap = vi.fn();
      mockedListen.mockImplementation(async (_eventName, handler) => {
        handler({
          event: "desktop://bootstrap",
          id: 1,
          payload: bootstrap,
        } as { event: string; id: number; payload: DesktopBootstrapState });
        return unsubscribe;
      });

      await expect(subscribeBootstrap(onBootstrap)).resolves.toBe(unsubscribe);
      expect(mockedListen).toHaveBeenCalledWith(
        "desktop://bootstrap",
        expect.any(Function),
      );
      expect(onBootstrap).toHaveBeenCalledWith(bootstrap);
    });
  });

  describe("join and host utility commands", () => {
    it("invokes start_host_session with the live host setup request", async () => {
      const request = sampleStartHostSessionRequest();
      const status = sampleHostSessionStatus();
      mockedInvoke.mockResolvedValue(status);

      await expect(startHostSession(request)).resolves.toEqual(status);
      expect(mockedInvoke).toHaveBeenCalledWith("start_host_session", { request });
    });

    it("invokes get_host_session_status and stop_host_session", async () => {
      const status = sampleHostSessionStatus();
      mockedInvoke.mockResolvedValueOnce(status).mockResolvedValueOnce(undefined);

      await expect(getHostSessionStatus()).resolves.toEqual(status);
      await expect(stopHostSession()).resolves.toBeUndefined();

      expect(mockedInvoke).toHaveBeenNthCalledWith(1, "get_host_session_status");
      expect(mockedInvoke).toHaveBeenNthCalledWith(2, "stop_host_session");
    });

    it("invokes the host lobby mutation commands", async () => {
      const seatRequest = sampleClaimLobbySeatRequest();
      const readyRequest = sampleSetLobbyReadyStateRequest();
      const status = sampleHostSessionStatus();
      mockedInvoke
        .mockResolvedValueOnce(status)
        .mockResolvedValueOnce(status)
        .mockResolvedValueOnce(status);

      await expect(hostClaimLobbySeat(seatRequest)).resolves.toEqual(status);
      await expect(hostSetLobbyReadyState(readyRequest)).resolves.toEqual(status);
      await expect(hostStartTournament()).resolves.toEqual(status);

      expect(mockedInvoke).toHaveBeenNthCalledWith(1, "host_claim_lobby_seat", {
        request: seatRequest,
      });
      expect(mockedInvoke).toHaveBeenNthCalledWith(2, "host_set_lobby_ready_state", {
        request: readyRequest,
      });
      expect(mockedInvoke).toHaveBeenNthCalledWith(3, "host_start_tournament");
    });

    it("invokes join_host_session, get_client_session_status, and leave_client_session", async () => {
      const request = sampleJoinHostSessionRequest();
      const status = sampleClientSessionStatus();
      mockedInvoke
        .mockResolvedValueOnce(status)
        .mockResolvedValueOnce(status)
        .mockResolvedValueOnce(undefined);

      await expect(joinHostSession(request)).resolves.toEqual(status);
      await expect(getClientSessionStatus()).resolves.toEqual(status);
      await expect(leaveClientSession()).resolves.toBeUndefined();

      expect(mockedInvoke).toHaveBeenNthCalledWith(1, "join_host_session", { request });
      expect(mockedInvoke).toHaveBeenNthCalledWith(2, "get_client_session_status");
      expect(mockedInvoke).toHaveBeenNthCalledWith(3, "leave_client_session");
    });

    it("invokes the client lobby mutation commands", async () => {
      const seatRequest = sampleClaimLobbySeatRequest();
      const readyRequest = sampleSetLobbyReadyStateRequest();
      const status = sampleClientSessionStatus();
      mockedInvoke
        .mockResolvedValueOnce(status)
        .mockResolvedValueOnce(status);

      await expect(clientClaimLobbySeat(seatRequest)).resolves.toEqual(status);
      await expect(clientSetLobbyReadyState(readyRequest)).resolves.toEqual(status);

      expect(mockedInvoke).toHaveBeenNthCalledWith(1, "client_claim_lobby_seat", {
        request: seatRequest,
      });
      expect(mockedInvoke).toHaveBeenNthCalledWith(2, "client_set_lobby_ready_state", {
        request: readyRequest,
      });
    });

    it("invokes validate_join_payload_input with the raw payload string", async () => {
      const payload = sampleJoinPayload();
      mockedInvoke.mockResolvedValue(payload);

      await expect(validateJoinPayloadInput("  pkr1_join  ")).resolves.toEqual(payload);
      expect(mockedInvoke).toHaveBeenCalledWith("validate_join_payload_input", {
        payload: "  pkr1_join  ",
      });
    });

    it("invokes resolve_host_lan_address when no browser override is present", async () => {
      mockedInvoke.mockResolvedValue("192.168.1.20");

      await expect(resolveHostLanAddress()).resolves.toBe("192.168.1.20");
      expect(mockedInvoke).toHaveBeenCalledWith("resolve_host_lan_address");
    });

    it("lets browser mocks override join and host utility commands", async () => {
      const startHostMock = vi.fn().mockResolvedValue(sampleHostSessionStatus());
      const getHostStatusMock = vi.fn().mockResolvedValue(sampleHostSessionStatus());
      const stopHostMock = vi.fn().mockResolvedValue(undefined);
      const hostClaimSeatMock = vi.fn().mockResolvedValue(sampleHostSessionStatus());
      const hostReadyMock = vi.fn().mockResolvedValue(sampleHostSessionStatus());
      const hostStartMock = vi.fn().mockResolvedValue(sampleHostSessionStatus());
      const joinHostMock = vi.fn().mockResolvedValue(sampleClientSessionStatus());
      const getClientStatusMock = vi.fn().mockResolvedValue(sampleClientSessionStatus());
      const leaveClientMock = vi.fn().mockResolvedValue(undefined);
      const clientClaimSeatMock = vi.fn().mockResolvedValue(sampleClientSessionStatus());
      const clientReadyMock = vi.fn().mockResolvedValue(sampleClientSessionStatus());
      const validateMock = vi.fn().mockResolvedValue(sampleJoinPayload());
      const hostIpMock = vi.fn().mockResolvedValue("10.0.0.4");
      setBrowserMocks({
        startHostSession: startHostMock,
        getHostSessionStatus: getHostStatusMock,
        stopHostSession: stopHostMock,
        hostClaimLobbySeat: hostClaimSeatMock,
        hostSetLobbyReadyState: hostReadyMock,
        hostStartTournament: hostStartMock,
        joinHostSession: joinHostMock,
        getClientSessionStatus: getClientStatusMock,
        leaveClientSession: leaveClientMock,
        clientClaimLobbySeat: clientClaimSeatMock,
        clientSetLobbyReadyState: clientReadyMock,
        validateJoinPayloadInput: validateMock,
        resolveHostLanAddress: hostIpMock,
      });

      await startHostSession(sampleStartHostSessionRequest());
      await getHostSessionStatus();
      await stopHostSession();
      await hostClaimLobbySeat(sampleClaimLobbySeatRequest());
      await hostSetLobbyReadyState(sampleSetLobbyReadyStateRequest());
      await hostStartTournament();
      await joinHostSession(sampleJoinHostSessionRequest());
      await getClientSessionStatus();
      await leaveClientSession();
      await clientClaimLobbySeat(sampleClaimLobbySeatRequest());
      await clientSetLobbyReadyState(sampleSetLobbyReadyStateRequest());
      await validateJoinPayloadInput("pkr1_mock");
      await resolveHostLanAddress();

      expect(startHostMock).toHaveBeenCalledWith(sampleStartHostSessionRequest());
      expect(getHostStatusMock).toHaveBeenCalledTimes(1);
      expect(stopHostMock).toHaveBeenCalledTimes(1);
      expect(hostClaimSeatMock).toHaveBeenCalledWith(sampleClaimLobbySeatRequest());
      expect(hostReadyMock).toHaveBeenCalledWith(sampleSetLobbyReadyStateRequest());
      expect(hostStartMock).toHaveBeenCalledTimes(1);
      expect(joinHostMock).toHaveBeenCalledWith(sampleJoinHostSessionRequest());
      expect(getClientStatusMock).toHaveBeenCalledTimes(1);
      expect(leaveClientMock).toHaveBeenCalledTimes(1);
      expect(clientClaimSeatMock).toHaveBeenCalledWith(sampleClaimLobbySeatRequest());
      expect(clientReadyMock).toHaveBeenCalledWith(sampleSetLobbyReadyStateRequest());
      expect(validateMock).toHaveBeenCalledWith("pkr1_mock");
      expect(hostIpMock).toHaveBeenCalledTimes(1);
      expect(mockedInvoke).not.toHaveBeenCalled();
    });
  });

  describe("table view commands", () => {
    it("invokes get_table_view and get_debug_state with the provided viewer mode", async () => {
      const tableView = sampleTableView();
      const debugState = sampleDebugState();
      mockedInvoke
        .mockResolvedValueOnce(tableView)
        .mockResolvedValueOnce(debugState);

      await expect(getTableView("observer")).resolves.toEqual(tableView);
      await expect(getDebugState("local")).resolves.toEqual(debugState);

      expect(mockedInvoke).toHaveBeenNthCalledWith(1, "get_table_view", {
        viewerMode: "observer",
      });
      expect(mockedInvoke).toHaveBeenNthCalledWith(2, "get_debug_state", {
        viewerMode: "local",
      });
    });

    it("lets browser mocks override table view commands", async () => {
      const getTableViewMock = vi.fn().mockResolvedValue(sampleTableView());
      const getDebugStateMock = vi.fn().mockResolvedValue(sampleDebugState());
      setBrowserMocks({
        getTableView: getTableViewMock,
        getDebugState: getDebugStateMock,
      });

      await getTableView("local");
      await getDebugState("observer");

      expect(getTableViewMock).toHaveBeenCalledWith("local");
      expect(getDebugStateMock).toHaveBeenCalledWith("observer");
      expect(mockedInvoke).not.toHaveBeenCalled();
    });
  });

  describe("submitTableAction", () => {
    it("invokes submit_table_action with unchanged viewer mode and action kind", async () => {
      const tableView = sampleTableView();
      mockedInvoke.mockResolvedValue(tableView);

      await expect(submitTableAction("local", "checkOrCall")).resolves.toEqual(
        tableView,
      );
      expect(mockedInvoke).toHaveBeenCalledWith("submit_table_action", {
        viewerMode: "local",
        actionKind: "checkOrCall",
        raiseToAmount: null,
      });
    });

    it("passes explicit raise amounts through unchanged", async () => {
      mockedInvoke.mockResolvedValue(sampleTableView());

      await submitTableAction("observer", "betOrRaise", 180);

      expect(mockedInvoke).toHaveBeenCalledWith("submit_table_action", {
        viewerMode: "observer",
        actionKind: "betOrRaise",
        raiseToAmount: 180,
      });
    });

    it("lets browser mocks override submit_table_action", async () => {
      const submitMock = vi.fn().mockResolvedValue(sampleTableView());
      setBrowserMocks({ submitTableAction: submitMock });

      await submitTableAction("local", "allIn", 200);

      expect(submitMock).toHaveBeenCalledWith("local", "allIn", 200);
      expect(mockedInvoke).not.toHaveBeenCalled();
    });
  });

  describe("launchAdditionalClientInstance", () => {
    it("invokes the exact command and normalizes omitted payloads to null", async () => {
      mockedInvoke.mockResolvedValue("child-a");

      await expect(launchAdditionalClientInstance()).resolves.toBe("child-a");
      expect(mockedInvoke).toHaveBeenCalledWith(
        "launch_additional_client_instance",
        { joinPayload: null },
      );
    });

    it("passes explicit join payloads through unchanged", async () => {
      mockedInvoke.mockResolvedValue("child-b");

      await launchAdditionalClientInstance("pkr1_join");

      expect(mockedInvoke).toHaveBeenCalledWith(
        "launch_additional_client_instance",
        { joinPayload: "pkr1_join" },
      );
    });

    it("lets browser mocks override launchAdditionalClientInstance", async () => {
      const launchMock = vi.fn().mockResolvedValue("child-mock");
      setBrowserMocks({ launchAdditionalClientInstance: launchMock });

      await expect(launchAdditionalClientInstance("pkr1_browser")).resolves.toBe(
        "child-mock",
      );
      expect(launchMock).toHaveBeenCalledWith("pkr1_browser");
      expect(mockedInvoke).not.toHaveBeenCalled();
    });
  });

  describe("environment and drift guards", () => {
    it("treats missing window as no browser mocks", async () => {
      const originalWindow = globalThis.window;
      mockedInvoke.mockResolvedValue(createBootstrap({ instanceId: "server" }));

      Object.defineProperty(globalThis, "window", {
        configurable: true,
        value: undefined,
      });

      try {
        await fetchBootstrapState();
      } finally {
        Object.defineProperty(globalThis, "window", {
          configurable: true,
          value: originalWindow,
        });
      }

      expect(mockedInvoke).toHaveBeenCalledWith("get_bootstrap_state");
    });

    it("ignores unrelated window keys when resolving browser mocks", async () => {
      Object.defineProperty(window, "unrelatedDesktopPokerKey", {
        configurable: true,
        value: { fetchBootstrapState: vi.fn() },
      });
      mockedInvoke.mockResolvedValue(createBootstrap({ instanceId: "plain" }));

      await fetchBootstrapState();

      expect(mockedInvoke).toHaveBeenCalledWith("get_bootstrap_state");
    });
  });
});
