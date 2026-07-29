import { describe, expect, it } from "vitest";
import contract from "../fixtures/desktop-contract.json";
import { createBootstrap } from "../test/fixtures";
import type {
  ClientSessionStatus,
  DebugInspectorState,
  HostRuntimeHealth,
  HostSessionStatus,
  LlmProviderConfig,
  LlmProviderSettings,
  NpcProfileListResult,
  TableViewSnapshot,
} from "./desktop";

function sortedKeys(value: object) {
  return Object.keys(value).sort();
}

function expectedKeys(name: keyof typeof contract) {
  return [...contract[name]].sort();
}

describe("desktop DTO contract", () => {
  it("matches the committed Rust/TypeScript key fixture", () => {
    const participant = {
      playerId: "player-a",
      displayName: "Alice",
      seatIndex: 0,
      isHost: false,
      isReady: true,
      connectionState: "connected",
      participantState: "seated",
    };

    const hostStatus: HostSessionStatus = {
      tournamentName: "Friday Night",
      tableName: "Main Table",
      tableId: "table-1",
      sessionEpoch: 1,
      advertisedHost: "192.168.1.10",
      hostPort: 43818,
      invite: "pkr1_fixture",
      phase: "readyCheck",
      activeSeatCount: 1,
      openSeatCount: 1,
      participants: [participant],
    };

    const clientStatus: ClientSessionStatus = {
      tournamentName: "Friday Night",
      tableName: "Main Table",
      tableId: "table-1",
      sessionEpoch: 1,
      hostAddress: "192.168.1.10",
      hostPort: 43818,
      localPlayerId: "player-a",
      phase: "readyCheck",
      activeSeatCount: 1,
      openSeatCount: 1,
      reconnecting: false,
      terminated: false,
      lastError: null,
      participants: [participant],
    };

    const tableView: TableViewSnapshot = {
      viewerMode: "local",
      sessionConnection: "normal",
      tournamentName: "Friday Night",
      tableName: "Main Table",
      tableId: "table-1",
      tournamentPhase: "running",
      phaseLabel: "Running",
      streetLabel: "Flop",
      blindLevelLabel: "10 / 20",
      currentHandNumber: 1,
      boardCards: [],
      potTotal: 30,
      actionOwnerLabel: "Alice",
      eliminationSummary: "",
      observerBanner: null,
      seats: [],
      standings: [],
      handHistory: [],
      eventFeed: [],
      actionTray: null,
    };

    const hostRuntimeHealth: HostRuntimeHealth = {
      acceptErrorCount: 0,
      streamTimeoutErrorCount: 0,
      tickAdvanceErrorCount: 0,
      publishErrorCount: 0,
      stateLockErrorCount: 0,
      streamCloneErrorCount: 0,
      clientRegistryErrorCount: 0,
      reconnectMarkErrorCount: 0,
      snapshotSyncErrorCount: 0,
      pendingJoinLimitRejectionCount: 0,
      connectedClientLimitRejectionCount: 0,
      lastError: null,
      lastSuccessfulTickMs: null,
      lastSuccessfulPublishMs: null,
    };

    const debugState: DebugInspectorState = {
      protocolLog: [],
      snapshotJson: "{}",
      currentSequence: 0,
      currentHandNumber: null,
      actionWindowSummary: null,
      launchHint: "hint",
      npcTiltLevels: {},
      lastLlmFallback: null,
      lastNpcActionError: null,
      hostRuntimeHealth,
    };

    const profileList: NpcProfileListResult = { profiles: [], errors: [] };
    const providerSettings: LlmProviderSettings = {
      provider: "embeddedLocal",
      endpointUrl: null,
      model: "/tmp/model.gguf",
    };
    const providerConfig: LlmProviderConfig = {
      ...providerSettings,
      apiKey: null,
    };

    expect(sortedKeys(createBootstrap())).toEqual(
      expectedKeys("DesktopBootstrapState"),
    );
    expect(sortedKeys(hostStatus)).toEqual(expectedKeys("HostSessionStatus"));
    expect(sortedKeys(clientStatus)).toEqual(
      expectedKeys("ClientSessionStatus"),
    );
    expect(sortedKeys(tableView)).toEqual(expectedKeys("TableViewSnapshot"));
    expect(sortedKeys(debugState)).toEqual(expectedKeys("DebugInspectorState"));
    expect(sortedKeys(debugState.hostRuntimeHealth ?? {})).toEqual(
      expectedKeys("HostRuntimeHealth"),
    );
    expect(sortedKeys(profileList)).toEqual(
      expectedKeys("NpcProfileListResult"),
    );
    expect(sortedKeys(providerSettings)).toEqual(
      expectedKeys("LlmProviderSettings"),
    );
    expect(sortedKeys(providerConfig)).toEqual(
      expectedKeys("LlmProviderConfig"),
    );
  });
});
