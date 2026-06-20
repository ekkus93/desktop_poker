import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getClientSessionStatus,
  getHostSessionStatus,
  hostSetLobbyReadyState,
  leaveClientSession,
  onSessionUpdate,
  stopHostSession,
  type ClientSessionStatus,
  type HostSessionStatus,
} from "../api/desktop";
import { storageKey } from "../app/shell";
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
    leaveClientSession: vi.fn(),
    onSessionUpdate: vi.fn().mockResolvedValue(() => {}),
    stopHostSession: vi.fn(),
  };
});

const mockedGetClientSessionStatus = vi.mocked(getClientSessionStatus);
const mockedGetHostSessionStatus = vi.mocked(getHostSessionStatus);
const mockedHostSetLobbyReadyState = vi.mocked(hostSetLobbyReadyState);
const mockedLeaveClientSession = vi.mocked(leaveClientSession);
const mockedOnSessionUpdate = vi.mocked(onSessionUpdate);
const mockedStopHostSession = vi.mocked(stopHostSession);

const { mockNavigate } = vi.hoisted(() => ({ mockNavigate: vi.fn() }));

vi.mock("react-router-dom", async () => {
  const actual =
    await vi.importActual<typeof import("react-router-dom")>(
      "react-router-dom",
    );

  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

function createClientSession(
  overrides: Partial<ClientSessionStatus> = {},
): ClientSessionStatus {
  return {
    tournamentName: "Friday Finals Live",
    tableName: "Main Table",
    tableId: "table-1",
    sessionEpoch: 42,
    hostAddress: "192.168.1.20",
    hostPort: 43818,
    localPlayerId: "player-client",
    phase: "waitingForPlayers",
    activeSeatCount: 1,
    openSeatCount: 5,
    reconnecting: false,
    terminated: false,
    lastError: null,
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
      {
        playerId: "player-client",
        displayName: "Client Bravo",
        seatIndex: 1,
        isHost: false,
        isReady: false,
        connectionState: "connected",
        participantState: "seated",
      },
    ],
    ...overrides,
  };
}

function createHostSession(
  overrides: Partial<HostSessionStatus> = {},
): HostSessionStatus {
  return {
    tournamentName: "Friday Finals Live",
    tableName: "Main Table",
    tableId: "table-1",
    sessionEpoch: 42,
    advertisedHost: "192.168.1.10",
    hostPort: 43818,
    invite: "pkr1_live_invite",
    phase: "waitingForPlayers",
    activeSeatCount: 2,
    openSeatCount: 4,
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
    ...overrides,
  };
}

describe("TournamentLobbyScreen", () => {
  beforeEach(() => {
    localStorage.clear();
    mockedGetClientSessionStatus.mockReset();
    mockedGetHostSessionStatus.mockReset();
    mockedHostSetLobbyReadyState.mockReset();
    mockedLeaveClientSession.mockReset();
    mockedLeaveClientSession.mockResolvedValue(undefined);
    mockedOnSessionUpdate.mockReset();
    mockedOnSessionUpdate.mockResolvedValue(() => {});
    mockedStopHostSession.mockReset();
    mockNavigate.mockReset();
    mockedGetClientSessionStatus.mockResolvedValue(null);
    mockedGetHostSessionStatus.mockResolvedValue(createHostSession());
    mockedHostSetLobbyReadyState.mockImplementation(async (request) => {
      const nextSession = createHostSession({
        participants: createHostSession().participants.map((participant) =>
          participant.playerId === "local-player"
            ? { ...participant, isReady: request.isReady }
            : {
                ...participant,
                isReady: request.isReady ? true : participant.isReady,
              },
        ),
        phase: request.isReady ? "readyCheck" : "waitingForPlayers",
      });
      mockedGetHostSessionStatus.mockResolvedValue(nextSession);
      return nextSession;
    });
  });

  it("shows a live-session loading state instead of shell-local fallback data", async () => {
    mockedGetHostSessionStatus.mockImplementation(
      () => new Promise<HostSessionStatus | null>(() => {}),
    );

    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    localStorage.setItem(
      storageKey(bootstrap.storageNamespace, "host-draft"),
      JSON.stringify({ tournamentName: "Stale Local Draft", maxPlayers: 10 }),
    );

    renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    expect(screen.getByText("Loading live lobby")).toBeTruthy();
    expect(screen.queryByText("Stale Local Draft")).toBeNull();
    expect(screen.queryByText("Open seat")).toBeNull();
  });

  it("renders runtime-backed lobby state instead of shell-local host draft data", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    localStorage.setItem(
      storageKey(bootstrap.storageNamespace, "host-draft"),
      JSON.stringify({ tournamentName: "Stale Local Draft", maxPlayers: 10 }),
    );

    renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    expect(await screen.findByText("Friday Finals Live")).toBeTruthy();
    expect(screen.getByText("6 seats")).toBeTruthy();
    expect(screen.getByText("4 open seats")).toBeTruthy();
    expect(screen.getByText("You: Waiting")).toBeTruthy();
    expect(screen.getByText("2 waiting")).toBeTruthy();
    expect(screen.queryByText("Stale Local Draft")).toBeNull();
  });

  it("updates the local ready badge from the live host mutation path", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    expect(await screen.findByText("You: Waiting")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "I'm ready" }));

    await waitFor(() => {
      expect(mockedHostSetLobbyReadyState).toHaveBeenCalledWith({
        isReady: true,
      });
    });
    expect(await screen.findByText("You: Ready")).toBeTruthy();
    expect(screen.getByText("Table: Ready")).toBeTruthy();
    expect(
      screen.getByRole("button", { name: "Start tournament" }),
    ).toBeTruthy();
  });

  it("shows recovery actions when host stops before the table goes live (B5)", async () => {
    // Capture the session-update callback so we can fire it manually
    let capturedSessionUpdateCallback: (() => void) | undefined;
    mockedOnSessionUpdate.mockImplementation((cb) => {
      capturedSessionUpdateCallback = cb;
      return Promise.resolve(() => {});
    });

    // First poll returns a session; subsequent calls return null (host stopped)
    mockedGetHostSessionStatus
      .mockResolvedValueOnce(createHostSession())
      .mockResolvedValue(null);

    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    // Wait for the initial lobby to render
    expect(await screen.findByText("Friday Finals Live")).toBeTruthy();

    // Fire session-update event to trigger a re-poll (which returns null → recovery)
    await waitFor(() => expect(capturedSessionUpdateCallback).toBeDefined());
    capturedSessionUpdateCallback?.();

    // Wait for recovery state to appear
    await waitFor(() => {
      expect(screen.getAllByText("Host stopped").length).toBeGreaterThan(0);
    });
    expect(screen.getByRole("button", { name: "Host again" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Return home" })).toBeTruthy();
  });

  it("keeps the leave dialog open with error and retry button after a backend failure (B7)", async () => {
    mockedStopHostSession.mockRejectedValue(new Error("Network error"));

    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    // Open the leave dialog — there are two "Close table" buttons (toolbar + dialog).
    // Click the first one (toolbar) to open the dialog.
    const allCloseButtons = await screen.findAllByRole("button", {
      name: "Close table",
    });
    fireEvent.click(allCloseButtons[0]);
    expect(screen.getByText("Close this table?")).toBeTruthy();

    // Attempt to leave via the dialog's own button (triggers the error)
    const dialogCloseButtons = screen.getAllByRole("button", {
      name: "Close table",
    });
    fireEvent.click(dialogCloseButtons[dialogCloseButtons.length - 1]);

    // Dialog should stay open with error and retry available
    await waitFor(() => {
      expect(screen.getAllByText(/network error/i).length).toBeGreaterThan(0);
    });
    expect(screen.getByText("Close this table?")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Stay here" })).toBeTruthy();
  });

  it("shows (AI) detail and Ready badge for NPC seat without a Take seat button", async () => {
    mockedGetHostSessionStatus.mockResolvedValue(
      createHostSession({
        activeSeatCount: 3,
        openSeatCount: 3,
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
          {
            playerId: "npc-seat-1",
            displayName: "Bot Beta",
            seatIndex: 1,
            isHost: false,
            isReady: true,
            connectionState: "connected",
            participantState: "seated",
          },
          {
            playerId: "player-c",
            displayName: "Maya",
            seatIndex: 2,
            isHost: false,
            isReady: false,
            connectionState: "connected",
            participantState: "seated",
          },
        ],
      }),
    );

    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    await screen.findByText("Bot Beta");
    expect(screen.getByText("(AI) · Always ready")).toBeTruthy();

    // NPC seat should show Ready badge
    const npcCard = screen.getByText("Bot Beta").closest("article");
    expect(npcCard?.textContent).toContain("Ready");

    // No "Take seat" button for NPC-occupied seats (they are filled, not open)
    expect(screen.queryByRole("button", { name: /take seat 2/i })).toBeNull();
  });

  it("does not count NPC seats in seatsStillWaiting", async () => {
    mockedGetHostSessionStatus.mockResolvedValue(
      createHostSession({
        activeSeatCount: 2,
        openSeatCount: 4,
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
          {
            playerId: "npc-seat-1",
            displayName: "Bot Beta",
            seatIndex: 1,
            isHost: false,
            isReady: true,
            connectionState: "connected",
            participantState: "seated",
          },
        ],
      }),
    );

    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    // Only the real local player (Host Alpha) is waiting — NPC should not count
    expect(await screen.findByText("1 waiting")).toBeTruthy();
  });

  it("stops polling and navigates to the error screen after 10 consecutive status errors (B6)", async () => {
    vi.useFakeTimers();
    mockedGetHostSessionStatus.mockRejectedValue(new Error("host unreachable"));
    mockedGetClientSessionStatus.mockResolvedValue(null);

    const bootstrap = createBootstrap();
    await act(async () => {
      renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
        bootstrap,
        initialEntries: ["/lobby"],
      });
    });

    // A handful of early failures stay under the error limit and must not navigate.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(15000);
    });
    expect(mockNavigate).not.toHaveBeenCalled();

    // Crossing the 10-error limit navigates to the dedicated error screen.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(100000);
    });
    expect(mockNavigate).toHaveBeenCalledWith("/errors", { replace: true });

    vi.useRealTimers();
  });

  it("keeps the lobby visible and shows the connection-slow banner after 3 consecutive errors (B6)", async () => {
    vi.useFakeTimers();
    // First poll establishes the session; subsequent polls fail transiently.
    mockedGetHostSessionStatus
      .mockResolvedValueOnce(createHostSession())
      .mockRejectedValue(new Error("host unreachable"));
    mockedGetClientSessionStatus.mockResolvedValue(null);

    const bootstrap = createBootstrap();
    await act(async () => {
      renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
        bootstrap,
        initialEntries: ["/lobby"],
      });
    });

    // Two failed polls stay on the normal interval with no banner yet.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10000);
    });
    expect(screen.queryByText(/connection slow/i)).toBeNull();

    // The third consecutive failure shows the slow banner. The banner only
    // exists inside the live-lobby view, so its presence proves the lobby stayed
    // on screen instead of dropping to the no-session/recovery view.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });
    expect(screen.getByText(/connection slow/i)).toBeTruthy();
    expect(mockNavigate).not.toHaveBeenCalled();

    vi.useRealTimers();
  });

  it("shows a reconnecting banner when clientSession.reconnecting is true", async () => {
    mockedGetHostSessionStatus.mockResolvedValue(null);
    mockedGetClientSessionStatus.mockResolvedValue(
      createClientSession({ reconnecting: true }),
    );

    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    await act(async () => {
      renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
        bootstrap,
        initialEntries: ["/lobby"],
      });
    });

    expect(screen.getByText(/reconnecting to host/i)).toBeTruthy();
    expect(screen.queryByText(/connection slow/i)).toBeNull();
  });

  it("shows a terminal disconnect error when clientSession.terminated is true and stops polling", async () => {
    vi.useFakeTimers();
    mockedGetHostSessionStatus.mockResolvedValue(null);
    mockedGetClientSessionStatus.mockResolvedValue(
      createClientSession({ terminated: true, lastError: "Disconnected from host" }),
    );

    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    await act(async () => {
      renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
        bootstrap,
        initialEntries: ["/lobby"],
      });
    });

    expect(screen.getByText(/disconnected from host/i)).toBeTruthy();
    expect(screen.queryByText(/reconnecting/i)).toBeNull();

    // Polling stops — no additional calls after the terminal state is detected.
    const callCountAfterTerminal = mockedGetClientSessionStatus.mock.calls.length;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(15000);
    });
    expect(mockedGetClientSessionStatus.mock.calls.length).toBe(
      callCountAfterTerminal,
    );

    vi.useRealTimers();
  });

  it("leaves a terminated client session and navigates home when the leave button is clicked", async () => {
    mockedGetHostSessionStatus.mockResolvedValue(null);
    mockedGetClientSessionStatus.mockResolvedValue(
      createClientSession({ terminated: true, lastError: "Disconnected from host" }),
    );
    mockedLeaveClientSession.mockResolvedValue(undefined);

    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    await act(async () => {
      renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
        bootstrap,
        initialEntries: ["/lobby"],
      });
    });

    expect(screen.getByText(/disconnected from host/i)).toBeTruthy();

    // The leave button is always visible — click it to open the leave modal.
    const [leaveButton] = screen.getAllByRole("button", {
      name: /close table|leave table/i,
    });
    await act(async () => {
      fireEvent.click(leaveButton);
    });

    // The modal dialog appears; the confirm button has the same label as the
    // original leave button so there are now two matching buttons. The second
    // (index 1) is the primary confirm button inside the dialog.
    const [, confirmButton] = await screen.findAllByRole("button", {
      name: /close table|leave table/i,
    });
    await act(async () => {
      fireEvent.click(confirmButton);
    });

    expect(mockedLeaveClientSession).toHaveBeenCalled();
    expect(mockNavigate).toHaveBeenCalledWith("/", { replace: true });
  });
});
