import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getClientSessionStatus,
  getHostSessionStatus,
  hostSetLobbyReadyState,
  onSessionUpdate,
  type HostSessionStatus,
} from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { TournamentLobbyScreen } from "./TournamentLobbyScreen";

vi.mock("../api/desktop", async () => {
  const actual =
    await vi.importActual<typeof import("../api/desktop")>("../api/desktop");

  return {
    ...actual,
    getClientSessionStatus: vi.fn(),
    getHostSessionStatus: vi.fn(),
    hostSetLobbyReadyState: vi.fn(),
    onSessionUpdate: vi.fn().mockResolvedValue(() => {}),
  };
});

const mockedGetClientSessionStatus = vi.mocked(getClientSessionStatus);
const mockedGetHostSessionStatus = vi.mocked(getHostSessionStatus);
const mockedHostSetLobbyReadyState = vi.mocked(hostSetLobbyReadyState);
const mockedOnSessionUpdate = vi.mocked(onSessionUpdate);

const { mockNavigate } = vi.hoisted(() => ({ mockNavigate: vi.fn() }));

vi.mock("react-router", async () => {
  const actual =
    await vi.importActual<typeof import("react-router")>("react-router");

  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

function createHostSession(
  localReady = false,
  phase: HostSessionStatus["phase"] = "waitingForPlayers",
): HostSessionStatus {
  return {
    tournamentName: "Authoritative Ready Test",
    tableName: "Main Table",
    tableId: "table-ready-test",
    sessionEpoch: 42,
    advertisedHost: "192.168.1.10",
    hostPort: 43818,
    invite: "pkr1_authoritative_ready",
    phase,
    activeSeatCount: 2,
    openSeatCount: 0,
    participants: [
      {
        playerId: "local-player",
        displayName: "Host Alpha",
        seatIndex: 0,
        isHost: true,
        isReady: localReady,
        connectionState: "connected",
        participantState: "seated",
      },
      {
        playerId: "player-b",
        displayName: "Client Bravo",
        seatIndex: 1,
        isHost: false,
        isReady: localReady,
        connectionState: "connected",
        participantState: "seated",
      },
    ],
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });
  return { promise, resolve, reject };
}

function renderLobby() {
  const bootstrap = createBootstrap({ debugToolsEnabled: false });
  renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
    bootstrap,
    initialEntries: ["/lobby"],
  });
}

describe("TournamentLobbyScreen authoritative ready transitions", () => {
  beforeEach(() => {
    localStorage.clear();
    mockNavigate.mockReset();
    mockedGetClientSessionStatus.mockReset();
    mockedGetClientSessionStatus.mockResolvedValue(null);
    mockedGetHostSessionStatus.mockReset();
    mockedGetHostSessionStatus.mockResolvedValue(createHostSession());
    mockedHostSetLobbyReadyState.mockReset();
    mockedOnSessionUpdate.mockReset();
    mockedOnSessionUpdate.mockResolvedValue(() => {});
  });

  it("keeps waiting state and Start disabled until the ready mutation is confirmed", async () => {
    const mutation = deferred<HostSessionStatus>();
    mockedHostSetLobbyReadyState.mockReturnValue(mutation.promise);
    renderLobby();

    expect(await screen.findByText("You: Waiting")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "I'm ready" }));

    await waitFor(() => {
      expect(mockedHostSetLobbyReadyState).toHaveBeenCalledWith({
        isReady: true,
      });
    });

    expect(screen.getByText("You: Marking ready…")).toBeTruthy();
    expect(screen.queryByText("You: Ready")).toBeNull();
    const pendingButton = screen.getByRole("button", {
      name: "Marking ready…",
    }) as HTMLButtonElement;
    expect(pendingButton.disabled).toBe(true);
    const pendingStart = screen.getByRole("button", {
      name: "Start tournament",
    }) as HTMLButtonElement;
    expect(pendingStart.disabled).toBe(true);
    expect(
      screen.getByText("Host Alpha").closest("article")?.textContent,
    ).toContain("Waiting");

    await act(async () => {
      mutation.resolve(createHostSession(true, "readyCheck"));
      await mutation.promise;
    });

    expect(await screen.findByText("You: Ready")).toBeTruthy();
    expect(screen.getByText("Table: Ready")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Undo ready" })).toBeTruthy();
    const confirmedStart = screen.getByRole("button", {
      name: "Start tournament",
    }) as HTMLButtonElement;
    expect(confirmedStart.disabled).toBe(false);
  });

  it("restores the authoritative waiting state when the backend rejects ready", async () => {
    const mutation = deferred<HostSessionStatus>();
    mockedHostSetLobbyReadyState.mockReturnValue(mutation.promise);
    renderLobby();

    expect(await screen.findByText("You: Waiting")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "I'm ready" }));
    expect(await screen.findByText("You: Marking ready…")).toBeTruthy();

    await act(async () => {
      mutation.reject(new Error("Ready state was rejected by the host"));
      try {
        await mutation.promise;
      } catch {
        // The component converts the rejected mutation into visible lobby state.
      }
    });

    expect(
      await screen.findByText("Ready state was rejected by the host"),
    ).toBeTruthy();
    expect(screen.getByText("You: Waiting")).toBeTruthy();
    expect(screen.queryByText("You: Ready")).toBeNull();
    const retryButton = screen.getByRole("button", {
      name: "I'm ready",
    }) as HTMLButtonElement;
    expect(retryButton.disabled).toBe(false);
    const startButton = screen.getByRole("button", {
      name: "Start tournament",
    }) as HTMLButtonElement;
    expect(startButton.disabled).toBe(true);
  });

  it("keeps confirmed ready state visible as pending until undo is acknowledged", async () => {
    const initialReady = createHostSession(true, "readyCheck");
    mockedGetHostSessionStatus.mockResolvedValue(initialReady);
    const mutation = deferred<HostSessionStatus>();
    mockedHostSetLobbyReadyState.mockReturnValue(mutation.promise);
    renderLobby();

    expect(await screen.findByText("You: Ready")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Undo ready" }));

    expect(await screen.findByText("You: Undoing ready…")).toBeTruthy();
    expect(
      screen.getByText("Host Alpha").closest("article")?.textContent,
    ).toContain("Ready");
    const pendingUndo = screen.getByRole("button", {
      name: "Undoing ready…",
    }) as HTMLButtonElement;
    expect(pendingUndo.disabled).toBe(true);

    await act(async () => {
      mutation.resolve(createHostSession(false, "waitingForPlayers"));
      await mutation.promise;
    });

    expect(await screen.findByText("You: Waiting")).toBeTruthy();
    expect(screen.queryByText("Table: Ready")).toBeNull();
    expect(screen.getByRole("button", { name: "I'm ready" })).toBeTruthy();
  });
});
