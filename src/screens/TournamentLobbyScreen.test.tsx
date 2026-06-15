import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getClientSessionStatus,
  getHostSessionStatus,
  hostSetLobbyReadyState,
  onSessionUpdate,
  stopHostSession,
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
    onSessionUpdate: vi.fn().mockResolvedValue(() => {}),
    stopHostSession: vi.fn(),
  };
});

const mockedGetClientSessionStatus = vi.mocked(getClientSessionStatus);
const mockedGetHostSessionStatus = vi.mocked(getHostSessionStatus);
const mockedHostSetLobbyReadyState = vi.mocked(hostSetLobbyReadyState);
const mockedOnSessionUpdate = vi.mocked(onSessionUpdate);
const mockedStopHostSession = vi.mocked(stopHostSession);

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
    mockedOnSessionUpdate.mockReset();
    mockedOnSessionUpdate.mockResolvedValue(() => {});
    mockedStopHostSession.mockReset();
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
});
