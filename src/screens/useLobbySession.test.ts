import { act, renderHook, waitFor } from "@testing-library/react";
import type { NavigateFunction } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  ClientSessionStatus,
  HostSessionParticipantView,
  HostSessionStatus,
} from "../api/desktop";
import {
  getClientSessionStatus,
  getHostSessionStatus,
  onSessionUpdate,
} from "../api/desktop";
import { buildLiveSeats, useLobbySession } from "./useLobbySession";

vi.mock("../api/desktop", async () => {
  const actual =
    await vi.importActual<typeof import("../api/desktop")>("../api/desktop");
  return {
    ...actual,
    getClientSessionStatus: vi.fn(),
    getHostSessionStatus: vi.fn(),
    onSessionUpdate: vi.fn(),
  };
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((callback, rejectPromise) => {
    resolve = callback;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

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

describe("useLobbySession refresh ordering", () => {
  let emitSessionUpdate: () => void;

  beforeEach(() => {
    vi.mocked(getHostSessionStatus).mockReset();
    vi.mocked(getHostSessionStatus).mockResolvedValue(null);
    vi.mocked(getClientSessionStatus).mockReset();
    vi.mocked(onSessionUpdate).mockReset();
    vi.mocked(onSessionUpdate).mockImplementation((callback) => {
      emitSessionUpdate = callback;
      return Promise.resolve(() => {});
    });
  });

  it("discards a slow initial refresh after a later event refresh succeeds", async () => {
    const old = deferred<ClientSessionStatus | null>();
    const newer = deferred<ClientSessionStatus | null>();
    vi.mocked(getClientSessionStatus)
      .mockImplementationOnce(() => old.promise)
      .mockImplementationOnce(() => newer.promise);
    const navigate = vi.fn() as unknown as NavigateFunction;
    const { result, unmount } = renderHook(() => useLobbySession(navigate));

    act(() => {
      emitSessionUpdate();
      newer.resolve({
        ...clientStatus([hostParticipant, pendingClient], 1, 1),
        phase: "running",
      });
    });
    await waitFor(() => {
      expect(result.current.clientSession?.phase).toBe("running");
    });
    await act(async () => {
      old.resolve(clientStatus([hostParticipant, pendingClient], 1, 1));
      await old.promise;
    });
    expect(result.current.clientSession?.phase).toBe("running");
    unmount();
  });

  it("discards a stale initial failure after a later event refresh succeeds", async () => {
    const old = deferred<ClientSessionStatus | null>();
    const newer = deferred<ClientSessionStatus | null>();
    vi.mocked(getClientSessionStatus)
      .mockImplementationOnce(() => old.promise)
      .mockImplementationOnce(() => newer.promise);
    const navigate = vi.fn() as unknown as NavigateFunction;
    const { result, unmount } = renderHook(() => useLobbySession(navigate));

    act(() => {
      emitSessionUpdate();
      newer.resolve({
        ...clientStatus([hostParticipant, pendingClient], 1, 1),
        phase: "running",
      });
    });
    await waitFor(() => {
      expect(result.current.clientSession?.phase).toBe("running");
    });
    await act(async () => {
      old.reject(new Error("stale request failed"));
      await old.promise.catch(() => {});
    });

    expect(result.current.clientSession?.phase).toBe("running");
    expect(result.current.connectionSlow).toBe(false);
    expect(navigate).not.toHaveBeenCalled();
    unmount();
  });

  it("preserves terminal status against later nonterminal refresh for the same session", async () => {
    const terminal = {
      ...clientStatus([hostParticipant, pendingClient], 1, 1),
      terminated: true,
      lastError: "host disconnected",
    };
    vi.mocked(getClientSessionStatus)
      .mockResolvedValueOnce(terminal)
      .mockResolvedValueOnce({
        ...terminal,
        terminated: false,
        reconnecting: false,
      })
      .mockResolvedValueOnce({
        ...terminal,
        tableId: "new-tournament",
        sessionEpoch: 2,
        terminated: false,
        lastError: null,
      });
    const navigate = vi.fn() as unknown as NavigateFunction;
    const { result, unmount } = renderHook(() => useLobbySession(navigate));
    await waitFor(() => expect(result.current.clientTerminated).toBe(true));
    act(() => emitSessionUpdate());
    await waitFor(() =>
      expect(getClientSessionStatus).toHaveBeenCalledTimes(2),
    );
    expect(result.current.clientTerminated).toBe(true);
    act(() => emitSessionUpdate());
    await waitFor(() =>
      expect(result.current.clientSession?.tableId).toBe("new-tournament"),
    );
    expect(result.current.clientTerminated).toBe(false);
    unmount();
  });
});
