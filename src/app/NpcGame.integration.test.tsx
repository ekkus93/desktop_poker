import { describe, expect, it, beforeEach } from "vitest";
import {
  TournamentController,
  TournamentError,
  ActionType,
  TournamentPhase,
  TournamentConfig,
  BlindSchedule,
  BlindLevel,
  NpcConfig,
  NpcStyle,
  RegisteredPlayer,
  PlayerIdentity,
} from "../api/desktop";

// Minimal mock identity for testing
function createMockIdentity(
  playerId: string,
  displayName: string,
  isHost: boolean = false,
): PlayerIdentity {
  return {
    playerId,
    displayName,
    signingPublicKey: "mock-signing-pubkey",
    encryptionPublicKey: "mock-encryption-pubkey",
    signingKeyFingerprint: "mock-fingerprint",
  };
}

// Minimal mock NpcConfig
function createMockNpc(seatIndex: number, style: NpcStyle = NpcStyle.Conservative): NpcConfig {
  return {
    displayName: `NPC Seat ${seatIndex}`,
    style,
    profile: null,
  };
}

// Minimal mock RegisteredPlayer
function createMockPlayer(
  playerId: string,
  seatIndex: number,
  isHost: boolean = false,
): RegisteredPlayer {
  return {
    identity: createMockIdentity(playerId, playerId, isHost),
    seatIndex,
    isHost,
    isReady: true,
  };
}

describe("NpcGame Integration", () => {
  it("creates a 2-player tournament with one human and one NPC", () => {
    const config = {
      tournamentName: "Human vs NPC Test",
      tableId: "test-table-1",
      maxPlayers: 2,
      startingStack: 1000,
      turnTimerSeconds: 30,
      blindSchedule: {
        levels: [
          {
            levelIndex: 0,
            label: "Level 1",
            smallBlind: 50,
            bigBlind: 100,
            ante: 10,
            durationSeconds: 60,
          },
        ],
      },
    } as TournamentConfig;

    const humanPlayer = createMockPlayer("human", 0, true);
    const npcPlayer = createMockPlayer("npc-0", 1, false);

    const controller = new TournamentController(
      config.tableId,
      config.sessionEpoch,
      config,
      [humanPlayer, npcPlayer],
    );

    expect(controller.state().phase).toBe(TournamentPhase.ReadyCheck);
    expect(controller.state().config.maxPlayers).toBe(2);
  });

  it("rejects tournament creation with fewer than 2 players", () => {
    const config = {
      tournamentName: "Test",
      tableId: "test-table-2",
      maxPlayers: 2,
      startingStack: 1000,
      turnTimerSeconds: 30,
      blindSchedule: {
        levels: [
          {
            levelIndex: 0,
            label: "Level 1",
            smallBlind: 50,
            bigBlind: 100,
            ante: 10,
            durationSeconds: 60,
          },
        ],
      },
    } as TournamentConfig;

    const controller = new TournamentController(
      config.tableId,
      config.sessionEpoch,
      config,
      [createMockPlayer("human", 0, true)],
    );

    expect(controller.state().phase).toBe(TournamentPhase.ReadyCheck);
    // Should be in ReadyCheck phase, waiting for second player
  });

  it("starts tournament successfully with 2 ready players", () => {
    const config = {
      tournamentName: "Test",
      tableId: "test-table-3",
      maxPlayers: 2,
      startingStack: 1000,
      turnTimerSeconds: 30,
      blindSchedule: {
        levels: [
          {
            levelIndex: 0,
            label: "Level 1",
            smallBlind: 50,
            bigBlind: 100,
            ante: 10,
            durationSeconds: 60,
          },
        ],
      },
    } as TournamentConfig;

    const humanPlayer = createMockPlayer("human", 0, true);
    const npcPlayer = createMockPlayer("npc-0", 1, false);

    const controller = new TournamentController(
      config.tableId,
      config.sessionEpoch,
      config,
      [humanPlayer, npcPlayer],
    );

    const now = Date.now();
    controller.start_tournament(now);

    expect(controller.state().phase).toBe(TournamentPhase.Running);
    expect(controller.state().seats[0].chipCount).toBe(1000);
    expect(controller.state().seats[1].chipCount).toBe(1000);
  });

  it("rejects tournament start with insufficient players (2-10 required)", () => {
    const config = {
      tournamentName: "Test",
      tableId: "test-table-4",
      maxPlayers: 2,
      startingStack: 1000,
      turnTimerSeconds: 30,
      blindSchedule: {
        levels: [
          {
            levelIndex: 0,
            label: "Level 1",
            smallBlind: 50,
            bigBlind: 100,
            ante: 10,
            durationSeconds: 60,
          },
        ],
      },
    } as TournamentConfig;

    const npcPlayer = createMockPlayer("npc-0", 1, false);

    const controller = new TournamentController(
      config.tableId,
      config.sessionEpoch,
      config,
      [npcPlayer],
    );

    const now = Date.now();
    const error = controller.start_tournament(now);

    expect(error.isOk()).toBe(false);
    // Should get an error about insufficient players
  });

  it("rejects tournament start when players are not ready", () => {
    const config = {
      tournamentName: "Test",
      tableId: "test-table-5",
      maxPlayers: 2,
      startingStack: 1000,
      turnTimerSeconds: 30,
      blindSchedule: {
        levels: [
          {
            levelIndex: 0,
            label: "Level 1",
            smallBlind: 50,
            bigBlind: 100,
            ante: 10,
            durationSeconds: 60,
          },
        ],
      },
    } as TournamentConfig;

    const npcPlayer = createMockPlayer("npc-0", 1, false, false);

    const controller = new TournamentController(
      config.tableId,
      config.sessionEpoch,
      config,
      [npcPlayer],
    );

    const now = Date.now();
    const error = controller.start_tournament(now);

    expect(error.isOk()).toBe(false);
  });
});
