import { describe, expect, it } from "vitest";
import type {
  ClientSessionStatus,
  HostSessionParticipantView,
  HostSessionStatus,
} from "../api/desktop";
import { buildLiveSeats } from "./useLobbySession";

const hostParticipant: HostSessionParticipantView = {
  playerId: "local-player",
  displayName: "Host",
  seatIndex: 0,
  isHost: true,
  isReady: false,
  connectionState: "connected",
  participantState: "seated",
};

const pendingClient: HostSessionParticipantView = {
  playerId: "remote-player",
  displayName: "Client",
  seatIndex: null,
  isHost: false,
  isReady: false,
  connectionState: "connected",
  participantState: "admitted",
};

function hostStatus(
  participants: HostSessionParticipantView[],
  activeSeatCount: number,
  openSeatCount: number,
): HostSessionStatus {
  return {
    tournamentName: "Runtime Test",
    tableName: "Main Table",
    tableId: "table-runtime-test",
    sessionEpoch: 1,
    advertisedHost: "127.0.0.1",
    hostPort: 43818,
    invite: "pkr1_test",
    phase: "waitingForPlayers",
    activeSeatCount,
    openSeatCount,
    participants,
  };
}

function clientStatus(
  participants: HostSessionParticipantView[],
  activeSeatCount: number,
  openSeatCount: number,
): ClientSessionStatus {
  return {
    tournamentName: "Runtime Test",
    tableName: "Main Table",
    tableId: "table-runtime-test",
    sessionEpoch: 1,
    hostAddress: "127.0.0.1",
    hostPort: 43818,
    localPlayerId: "remote-player",
    phase: "waitingForPlayers",
    activeSeatCount,
    openSeatCount,
    reconnecting: false,
    terminated: false,
    lastError: null,
    participants,
  };
}

describe("buildLiveSeats", () => {
  it("keeps an authoritative open seat available for an admitted unseated client", () => {
    const seats = buildLiveSeats(
      hostStatus([hostParticipant, pendingClient], 1, 1),
    );

    expect(seats).toEqual([
      expect.objectContaining({
        seatIndex: 1,
        label: "You",
        kind: "host",
        isLocal: true,
      }),
      expect.objectContaining({
        seatIndex: 2,
        label: "Open seat",
        kind: "open",
        isLocal: false,
      }),
    ]);
  });

  it("does not turn a pending local client into a fake occupied seat", () => {
    const seats = buildLiveSeats(
      clientStatus([hostParticipant, pendingClient], 1, 1),
    );

    expect(seats).toEqual([
      expect.objectContaining({
        seatIndex: 1,
        label: "Host",
        kind: "host",
        isLocal: false,
      }),
      expect.objectContaining({
        seatIndex: 2,
        label: "Open seat",
        kind: "open",
        isLocal: false,
      }),
    ]);
    expect(seats.some((seat) => seat.isLocal)).toBe(false);
  });

  it("distinguishes a seated client from the host", () => {
    const seatedClient: HostSessionParticipantView = {
      ...pendingClient,
      seatIndex: 1,
      participantState: "seated",
    };

    const seats = buildLiveSeats(
      clientStatus([hostParticipant, seatedClient], 2, 0),
    );

    expect(seats[0]).toEqual(
      expect.objectContaining({ kind: "host", isLocal: false }),
    );
    expect(seats[1]).toEqual(
      expect.objectContaining({
        label: "You",
        kind: "player",
        isLocal: true,
      }),
    );
  });
});
