import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  addNpcPlayers,
  clientClaimLobbySeat,
  clientSetLobbyReadyState,
  getClientSessionStatus,
  fetchBootstrapState,
  getHostSessionStatus,
  getDebugState,
  getTableView,
  listNpcProfiles,
  hostClaimLobbySeat,
  leaveClientSession,
  hostSetLobbyReadyState,
  hostStartTournament,
  joinHostSession,
  launchAdditionalClientInstance,
  onSessionUpdate,
  onTableUpdate,
  resolveHostLanAddress,
  stopHostSession,
  startHostSession,
  submitTableAction,
  subscribeBootstrap,
  validateJoinPayloadInput,
} from "../api/desktop";
import {
  createAppBootstrap,
  createParsedJoinPayload,
  createTableViewSnapshot,
} from "../test/appIntegrationFixtures";
import { storageKey } from "./shell";
import { DesktopBootstrapProvider } from "./DesktopBootstrapProvider";
import { AppShell } from "./AppShell";

vi.mock("../api/desktop", async () => {
  const actual =
    await vi.importActual<typeof import("../api/desktop")>("../api/desktop");

  return {
    ...actual,
    fetchBootstrapState: vi.fn(),
    addNpcPlayers: vi.fn(),
    listNpcProfiles: vi.fn(),
    clientClaimLobbySeat: vi.fn(),
    clientSetLobbyReadyState: vi.fn(),
    getClientSessionStatus: vi.fn(),
    getHostSessionStatus: vi.fn(),
    subscribeBootstrap: vi.fn(),
    getDebugState: vi.fn(),
    hostClaimLobbySeat: vi.fn(),
    leaveClientSession: vi.fn(),
    hostSetLobbyReadyState: vi.fn(),
    hostStartTournament: vi.fn(),
    joinHostSession: vi.fn(),
    launchAdditionalClientInstance: vi.fn(),
    onSessionUpdate: vi.fn().mockResolvedValue(() => {}),
    onTableUpdate: vi.fn().mockResolvedValue(() => {}),
    resolveHostLanAddress: vi.fn(),
    stopHostSession: vi.fn(),
    startHostSession: vi.fn(),
    validateJoinPayloadInput: vi.fn(),
    getTableView: vi.fn(),
    submitTableAction: vi.fn(),
  };
});

const mockedFetchBootstrapState = vi.mocked(fetchBootstrapState);
const mockedAddNpcPlayers = vi.mocked(addNpcPlayers);
const mockedListNpcProfiles = vi.mocked(listNpcProfiles);
const mockedClientClaimLobbySeat = vi.mocked(clientClaimLobbySeat);
const mockedClientSetLobbyReadyState = vi.mocked(clientSetLobbyReadyState);
const mockedGetClientSessionStatus = vi.mocked(getClientSessionStatus);
const mockedGetHostSessionStatus = vi.mocked(getHostSessionStatus);
const mockedSubscribeBootstrap = vi.mocked(subscribeBootstrap);
const mockedGetDebugState = vi.mocked(getDebugState);
const mockedHostClaimLobbySeat = vi.mocked(hostClaimLobbySeat);
const mockedLeaveClientSession = vi.mocked(leaveClientSession);
const mockedHostSetLobbyReadyState = vi.mocked(hostSetLobbyReadyState);
const mockedHostStartTournament = vi.mocked(hostStartTournament);
const mockedJoinHostSession = vi.mocked(joinHostSession);
const mockedLaunchAdditionalClientInstance = vi.mocked(
  launchAdditionalClientInstance,
);
const mockedOnSessionUpdate = vi.mocked(onSessionUpdate);
const mockedOnTableUpdate = vi.mocked(onTableUpdate);
const mockedResolveHostLanAddress = vi.mocked(resolveHostLanAddress);
const mockedStopHostSession = vi.mocked(stopHostSession);
const mockedStartHostSession = vi.mocked(startHostSession);
const mockedValidateJoinPayloadInput = vi.mocked(validateJoinPayloadInput);
const mockedGetTableView = vi.mocked(getTableView);
const mockedSubmitTableAction = vi.mocked(submitTableAction);
const clipboardWriteText = vi.fn();
let bootstrapSubscriptionHandler:
  | ((bootstrap: ReturnType<typeof createAppBootstrap>) => void)
  | undefined;
let currentHostSession: Awaited<ReturnType<typeof startHostSession>> | null;
let currentClientSession: Awaited<ReturnType<typeof joinHostSession>> | null;

function buildHostSessionStatus(request: {
  tournamentName: string;
  maxPlayers?: number;
  displayName?: string;
}) {
  const maxPlayers = request.maxPlayers ?? 6;

  return {
    tournamentName: request.tournamentName,
    tableName: "Main Table",
    tableId: "table-1",
    sessionEpoch: 42,
    advertisedHost: "192.168.1.10",
    hostPort: 43818,
    invite: "pkr1_host_invite",
    phase: "waitingForPlayers",
    activeSeatCount: 1,
    openSeatCount: maxPlayers - 1,
    participants: [
      {
        playerId: "local-player",
        displayName: request.displayName ?? "Player test-instance",
        seatIndex: 0,
        isHost: true,
        isReady: false,
        connectionState: "connected",
        participantState: "seated",
      },
    ],
  };
}

function buildClientSessionStatus() {
  return {
    tournamentName: currentHostSession?.tournamentName ?? "Friday Night",
    tableName: "Main Table",
    tableId: "table-1",
    sessionEpoch: 42,
    hostAddress: "192.168.1.10",
    hostPort: 43818,
    localPlayerId: "player-test-instance",
    phase: "waitingForPlayers",
    activeSeatCount: 1,
    openSeatCount: 5,
    reconnecting: false,
    lastError: null,
    participants: [
      {
        playerId: "local-player",
        displayName:
          currentHostSession?.participants[0]?.displayName ?? "Host Alpha",
        seatIndex: 0,
        isHost: true,
        isReady: false,
        connectionState: "connected",
        participantState: "seated",
      },
      {
        playerId: "player-test-instance",
        displayName: "Player test-instance",
        seatIndex: null,
        isHost: false,
        isReady: false,
        connectionState: "connected",
        participantState: "admitted",
      },
    ],
  };
}

function syncLiveSessions(
  participants: NonNullable<typeof currentHostSession>["participants"],
  forcedPhase?: string,
) {
  const totalSeats = currentHostSession
    ? currentHostSession.activeSeatCount + currentHostSession.openSeatCount
    : currentClientSession
      ? currentClientSession.activeSeatCount +
        currentClientSession.openSeatCount
      : 6;
  const activeSeatCount = participants.filter(
    (participant) => participant.seatIndex !== null,
  ).length;
  const phase =
    forcedPhase ??
    (activeSeatCount >= 2 &&
    participants
      .filter((participant) => participant.seatIndex !== null)
      .every((participant) => participant.isReady)
      ? "readyCheck"
      : "waitingForPlayers");

  if (currentHostSession) {
    currentHostSession = {
      ...currentHostSession,
      participants,
      activeSeatCount,
      openSeatCount: totalSeats - activeSeatCount,
      phase,
    };
  }

  if (currentClientSession) {
    currentClientSession = {
      ...currentClientSession,
      tournamentName:
        currentHostSession?.tournamentName ??
        currentClientSession.tournamentName,
      participants,
      activeSeatCount,
      openSeatCount: totalSeats - activeSeatCount,
      phase,
    };
  }
}

function renderAppShell(
  initialEntry: string,
  bootstrap = createAppBootstrap(),
  options?: { allowImplicitTableSession?: boolean },
) {
  if (
    options?.allowImplicitTableSession !== false &&
    initialEntry === "/table" &&
    !currentHostSession &&
    !currentClientSession
  ) {
    currentHostSession = {
      ...buildHostSessionStatus({
        tournamentName: "Friday Night",
      }),
      phase: "running",
    };
  }

  mockedFetchBootstrapState.mockResolvedValue(bootstrap);
  mockedSubscribeBootstrap.mockImplementation(async (onBootstrap) => {
    bootstrapSubscriptionHandler = onBootstrap;
    return () => {
      bootstrapSubscriptionHandler = undefined;
    };
  });

  return render(
    <MemoryRouter initialEntries={[initialEntry]}>
      <DesktopBootstrapProvider>
        <AppShell />
      </DesktopBootstrapProvider>
    </MemoryRouter>,
  );
}

describe("AppShell integration", () => {
  beforeEach(() => {
    localStorage.clear();
    bootstrapSubscriptionHandler = undefined;
    currentHostSession = null;
    currentClientSession = null;
    mockedFetchBootstrapState.mockReset();
    mockedAddNpcPlayers.mockReset();
    mockedListNpcProfiles.mockReset();
    mockedListNpcProfiles.mockResolvedValue([]);
    mockedClientClaimLobbySeat.mockReset();
    mockedClientSetLobbyReadyState.mockReset();
    mockedGetClientSessionStatus.mockReset();
    mockedGetHostSessionStatus.mockReset();
    mockedSubscribeBootstrap.mockReset();
    mockedGetDebugState.mockReset();
    mockedHostClaimLobbySeat.mockReset();
    mockedLeaveClientSession.mockReset();
    mockedHostSetLobbyReadyState.mockReset();
    mockedHostStartTournament.mockReset();
    mockedJoinHostSession.mockReset();
    mockedLaunchAdditionalClientInstance.mockReset();
    mockedOnSessionUpdate.mockReset();
    mockedOnSessionUpdate.mockResolvedValue(() => {});
    mockedOnTableUpdate.mockReset();
    mockedOnTableUpdate.mockResolvedValue(() => {});
    mockedResolveHostLanAddress.mockReset();
    mockedStopHostSession.mockReset();
    mockedStartHostSession.mockReset();
    mockedValidateJoinPayloadInput.mockReset();
    mockedGetTableView.mockReset();
    mockedSubmitTableAction.mockReset();
    clipboardWriteText.mockReset();
    mockedGetClientSessionStatus.mockImplementation(
      async () => currentClientSession,
    );
    mockedGetHostSessionStatus.mockImplementation(
      async () => currentHostSession,
    );
    mockedStartHostSession.mockImplementation(async (request) => {
      currentHostSession = buildHostSessionStatus({
        tournamentName: request.tournamentName,
        maxPlayers: request.maxPlayers,
        displayName: request.displayName,
      });
      return currentHostSession;
    });
    mockedAddNpcPlayers.mockImplementation(async ({ npcs }) => {
      if (!currentHostSession) {
        throw new Error("No active host session");
      }

      // Seat each NPC right after the host, mirroring how the backend assigns
      // `npc-seat-{index}` ids and marks NPCs ready.
      const npcParticipants = npcs.map((npc, index) => ({
        playerId: `npc-seat-${index + 1}`,
        displayName: npc.displayName,
        seatIndex: index + 1,
        isHost: false,
        isReady: true,
        connectionState: "connected",
        participantState: "seated",
      }));
      syncLiveSessions([
        ...currentHostSession.participants,
        ...npcParticipants,
      ]);
      return currentHostSession as NonNullable<typeof currentHostSession>;
    });
    mockedJoinHostSession.mockImplementation(async () => {
      currentClientSession = buildClientSessionStatus();
      if (currentHostSession) {
        syncLiveSessions(currentClientSession.participants);
      }
      return currentClientSession;
    });
    mockedHostClaimLobbySeat.mockImplementation(async (request) => {
      if (!currentHostSession) {
        throw new Error("No active host session");
      }

      const nextParticipants = currentHostSession.participants.map(
        (participant) =>
          participant.playerId === "local-player"
            ? {
                ...participant,
                seatIndex: request.seatIndex,
                isReady: false,
                participantState: "seated",
              }
            : participant,
      );
      syncLiveSessions(nextParticipants);
      return currentHostSession;
    });
    mockedHostSetLobbyReadyState.mockImplementation(async (request) => {
      if (!currentHostSession) {
        throw new Error("No active host session");
      }

      const nextParticipants = currentHostSession.participants.map(
        (participant) =>
          participant.playerId === "local-player"
            ? {
                ...participant,
                isReady: request.isReady,
                participantState: "seated",
              }
            : participant,
      );
      syncLiveSessions(nextParticipants);
      return currentHostSession;
    });
    mockedHostStartTournament.mockImplementation(async () => {
      if (!currentHostSession) {
        throw new Error("No active host session");
      }

      syncLiveSessions(currentHostSession.participants, "running");
      return currentHostSession;
    });
    mockedStopHostSession.mockImplementation(async () => {
      currentHostSession = null;
    });
    mockedLeaveClientSession.mockImplementation(async () => {
      currentClientSession = null;
    });
    mockedClientClaimLobbySeat.mockImplementation(async (request) => {
      if (!currentClientSession) {
        throw new Error("No active client session");
      }

      const nextParticipants = currentClientSession.participants.map(
        (participant) =>
          participant.playerId === "player-test-instance"
            ? {
                ...participant,
                seatIndex: request.seatIndex,
                isReady: false,
                participantState: "seated",
              }
            : participant,
      );
      syncLiveSessions(nextParticipants);
      return currentClientSession;
    });
    mockedClientSetLobbyReadyState.mockImplementation(async (request) => {
      if (!currentClientSession) {
        throw new Error("No active client session");
      }

      const nextParticipants = currentClientSession.participants.map(
        (participant) =>
          participant.playerId === "player-test-instance"
            ? {
                ...participant,
                isReady: request.isReady,
                participantState: "seated",
              }
            : participant,
      );
      syncLiveSessions(nextParticipants);
      return currentClientSession;
    });
    mockedResolveHostLanAddress.mockResolvedValue("192.168.1.10");
    mockedValidateJoinPayloadInput.mockResolvedValue(createParsedJoinPayload());
    mockedGetDebugState.mockResolvedValue({
      protocolLog: [],
      snapshotJson: "{}",
      currentSequence: 17,
      currentHandNumber: 9,
      actionWindowSummary:
        "You · check or bet · min 60 · max 1520 · legal Fold, Check, Bet",
      launchHint:
        "Spawn another debug client with its own storage namespace, or attach a copied pkr1_ payload to exercise local multi-instance join handoff.",
      npcTiltLevels: {},
    });
    mockedLaunchAdditionalClientInstance.mockResolvedValue("debug-child-1");
    mockedGetTableView.mockImplementation(async (viewerMode) =>
      createTableViewSnapshot({
        viewerMode,
        observerBanner:
          viewerMode === "observer"
            ? "Observer mode uses the public projector only: no private hole cards and no actions."
            : null,
        actionTray:
          viewerMode === "observer"
            ? null
            : createTableViewSnapshot().actionTray,
      }),
    );
    mockedSubmitTableAction.mockImplementation(async () =>
      createTableViewSnapshot({
        streetLabel: "Turn",
        potTotal: 240,
        handHistory: [
          ...createTableViewSnapshot().handHistory,
          {
            handNumber: 9,
            summary: "You won 240 chip(s).",
            potTotal: 240,
            winningPlayers: ["You"],
            eliminatedPlayers: [],
            boardCards: [],
          },
        ],
      }),
    );
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: clipboardWriteText,
      },
    });
  });

  it("moves from home to host setup and keeps the edited host draft in the lobby shell", async () => {
    renderAppShell("/");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();

    fireEvent.click(screen.getByRole("link", { name: "Host Tournament" }));

    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Host Tournament Setup",
      }),
    ).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Tournament name"), {
      target: { value: "Friday Finals" },
    });

    expect(
      screen.getByRole("region", { name: /host share summary/i }),
    ).toBeTruthy();
    expect(screen.getByText("Friday Finals")).toBeTruthy();
    expect(screen.getByText(/192\.168\.1\.10:43818/i)).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Start hosting" }));
    await waitFor(() => {
      expect(mockedStartHostSession).toHaveBeenCalled();
    });
    fireEvent.click(screen.getByRole("link", { name: "Continue to lobby" }));

    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Lobby",
      }),
    ).toBeTruthy();
    expect(await screen.findByText("Friday Finals")).toBeTruthy();
    expect(await screen.findByText("Seat 1")).toBeTruthy();
    expect(await screen.findByText("Seat 6")).toBeTruthy();
    expect(screen.getAllByText("Friday Finals").length).toBeGreaterThan(0);
  });

  it("hosts a table with one human and one NPC and shows both seats in the lobby", async () => {
    renderAppShell("/");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();

    fireEvent.click(screen.getByRole("link", { name: "Host Tournament" }));

    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Host Tournament Setup",
      }),
    ).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Tournament name"), {
      target: { value: "Heads-Up vs Bot" },
    });

    // Compose a "1 human (host) + 1 NPC" table.
    fireEvent.change(await screen.findByLabelText(/npc players/i), {
      target: { value: "1" },
    });

    fireEvent.click(screen.getByRole("button", { name: "Start hosting" }));

    await waitFor(() => {
      expect(mockedStartHostSession).toHaveBeenCalled();
    });
    await waitFor(() => {
      expect(mockedAddNpcPlayers).toHaveBeenCalledTimes(1);
    });
    const npcRequest = mockedAddNpcPlayers.mock.calls[0][0];
    expect(npcRequest.npcs).toHaveLength(1);
    expect(npcRequest.npcs[0].displayName).toBe("Bot Alpha");

    fireEvent.click(
      await screen.findByRole("link", { name: "Continue to lobby" }),
    );

    expect(
      await screen.findByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();

    // Both the human host and the single NPC are seated in the live lobby.
    expect((await screen.findAllByText("You")).length).toBeGreaterThan(0);
    expect(await screen.findByText("Bot Alpha")).toBeTruthy();
    expect(screen.getByText("(AI) · Always ready")).toBeTruthy();

    // The NPC is auto-ready, so only the human host counts as still waiting.
    expect(await screen.findByText("1 waiting")).toBeTruthy();
  });

  it("keeps a join flow in the lobby until a real table is running", async () => {
    renderAppShell("/join");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Join Tournament" }),
    ).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Invite"), {
      target: { value: "pkr1_good" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Check invite" }));

    expect(await screen.findByLabelText("Invite preview")).toBeTruthy();
    expect(screen.getAllByText("Friday Night").length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole("button", { name: "Continue to lobby" }));

    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Lobby",
      }),
    ).toBeTruthy();

    expect(
      (
        await screen.findByRole("button", { name: "Start tournament" })
      ).hasAttribute("disabled"),
    ).toBe(true);
    await waitFor(() => {
      expect(screen.queryByRole("button", { name: "I'm ready" })).toBeNull();
    });
  });

  it("lets a joined client claim a live seat and toggle ready through the lobby runtime", async () => {
    renderAppShell("/join");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Join Tournament" }),
    ).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Invite"), {
      target: { value: "pkr1_good" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Check invite" }));
    fireEvent.click(
      await screen.findByRole("button", { name: "Continue to lobby" }),
    );

    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Lobby",
      }),
    ).toBeTruthy();

    fireEvent.click(
      (await screen.findAllByRole("button", { name: "Take seat" }))[0],
    );

    await waitFor(() => {
      expect(mockedClientClaimLobbySeat).toHaveBeenCalledWith({ seatIndex: 2 });
    });
    expect(
      await screen.findByRole("button", { name: "I'm ready" }),
    ).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "I'm ready" }));

    await waitFor(() => {
      expect(mockedClientSetLobbyReadyState).toHaveBeenCalledWith({
        isReady: true,
      });
    });
    expect(screen.getByText("You: Ready")).toBeTruthy();
  });

  it("enables the host start control only when the live lobby is authoritatively ready", async () => {
    currentHostSession = buildHostSessionStatus({
      tournamentName: "Friday Night",
      displayName: "Host Alpha",
    });
    syncLiveSessions(
      [
        {
          playerId: "local-player",
          displayName: "Host Alpha",
          seatIndex: 0,
          isHost: true,
          isReady: true,
          connectionState: "connected",
          participantState: "seated",
        },
        {
          playerId: "player-test-instance",
          displayName: "Player test-instance",
          seatIndex: 1,
          isHost: false,
          isReady: true,
          connectionState: "connected",
          participantState: "seated",
        },
      ],
      "readyCheck",
    );

    renderAppShell("/lobby");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();
    await waitFor(() => {
      expect(
        screen
          .getByRole("button", { name: "Start tournament" })
          .hasAttribute("disabled"),
      ).toBe(false);
    });

    fireEvent.click(screen.getByRole("button", { name: "Start tournament" }));

    await waitFor(() => {
      expect(mockedHostStartTournament).toHaveBeenCalledTimes(1);
    });
    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
  });

  it("keeps the lobby in place when the authoritative host start rejects", async () => {
    currentHostSession = buildHostSessionStatus({
      tournamentName: "Friday Night",
      displayName: "Host Alpha",
    });
    syncLiveSessions(
      [
        {
          playerId: "local-player",
          displayName: "Host Alpha",
          seatIndex: 0,
          isHost: true,
          isReady: true,
          connectionState: "connected",
          participantState: "seated",
        },
        {
          playerId: "player-test-instance",
          displayName: "Player test-instance",
          seatIndex: 1,
          isHost: false,
          isReady: true,
          connectionState: "connected",
          participantState: "seated",
        },
      ],
      "readyCheck",
    );
    mockedHostStartTournament.mockRejectedValueOnce(
      new Error("host start rejected"),
    );

    renderAppShell("/lobby");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();
    await waitFor(() => {
      expect(
        screen
          .getByRole("button", { name: "Start tournament" })
          .hasAttribute("disabled"),
      ).toBe(false);
    });

    fireEvent.click(screen.getByRole("button", { name: "Start tournament" }));

    expect(await screen.findByText("host start rejected")).toBeTruthy();
    expect(
      screen.queryByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeNull();
    expect(
      screen.getByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();
  });

  it("shows a host recovery path when the live host session stops before play starts", async () => {
    // Capture the session-update callback so we can fire it manually
    let capturedSessionUpdateCallback: (() => void) | undefined;
    mockedOnSessionUpdate.mockImplementation((cb) => {
      capturedSessionUpdateCallback = cb;
      return Promise.resolve(() => {});
    });

    currentHostSession = buildHostSessionStatus({
      tournamentName: "Friday Night",
      displayName: "Host Alpha",
    });

    renderAppShell("/lobby");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();

    // The generic Lobby heading can render before the live host session has
    // been loaded. Wait for host-only lobby controls before simulating the
    // session disappearing, otherwise the recovery branch has no prior live
    // host session to recover from and the app correctly shows the generic
    // unavailable lobby state instead.
    expect(
      await screen.findByRole("button", { name: "Start tournament" }),
    ).toBeTruthy();
    await waitFor(() => {
      expect(screen.getAllByText("Friday Night").length).toBeGreaterThan(0);
    });

    // Simulate host session stopping and fire a session-update event to trigger re-poll
    currentHostSession = null;
    await waitFor(() => expect(capturedSessionUpdateCallback).toBeDefined(), {
      timeout: 2000,
    });
    // Wrap in act to flush the async Promise.all inside the callback
    await act(async () => {
      capturedSessionUpdateCallback?.();
      // Allow the microtask queue to drain so the .then() inside the callback can run
      await Promise.resolve();
    });

    await waitFor(
      () => {
        expect(
          screen.getByText(/hosting stopped before the table went live/i),
        ).toBeTruthy();
      },
      { timeout: 5000 },
    );
    expect(screen.getByRole("button", { name: "Host again" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Return home" })).toBeTruthy();
    expect(
      screen.queryByRole("button", { name: "Start tournament" }),
    ).toBeNull();
  }, 10_000);

  it("moves a joined client from the lobby to the table when the live session starts", async () => {
    currentClientSession = buildClientSessionStatus();
    syncLiveSessions(
      [
        {
          playerId: "local-player",
          displayName: "Host Alpha",
          seatIndex: 0,
          isHost: true,
          isReady: true,
          connectionState: "connected",
          participantState: "active",
        },
        {
          playerId: "player-test-instance",
          displayName: "Player test-instance",
          seatIndex: 1,
          isHost: false,
          isReady: true,
          connectionState: "connected",
          participantState: "active",
        },
      ],
      "running",
    );

    renderAppShell("/lobby");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
  });

  it("creates, copies, and joins a tournament invite across host and join flows", async () => {
    const hostRender = renderAppShell("/host");

    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Host Tournament Setup",
      }),
    ).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Tournament name"), {
      target: { value: "Invite Finals" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Start hosting" }));
    await screen.findByText(/ready on 192\.168\.1\.10/i);
    fireEvent.click(screen.getByRole("button", { name: "Copy invite" }));

    await waitFor(() => {
      expect(mockedStartHostSession).toHaveBeenCalledWith({
        hostAddress: "192.168.1.10",
        hostPort: 43818,
        tournamentName: "Invite Finals",
        maxPlayers: 6,
        startingStack: 1500,
        blindPresetId: "normal",
        turnTimerSeconds: 30,
        displayName: "Player test-instance",
      });
      expect(clipboardWriteText).toHaveBeenCalledWith("pkr1_host_invite");
    });

    hostRender.unmount();

    renderAppShell("/join");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Join Tournament" }),
    ).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Invite"), {
      target: { value: "pkr1_host_invite" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Check invite" }));

    expect(await screen.findByLabelText("Invite preview")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Continue to lobby" }));

    expect(
      await screen.findByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();
    expect(mockedJoinHostSession).toHaveBeenCalledWith({
      joinPayload: "pkr1_host_invite",
      displayName: "Player test-instance",
    });
    expect(await screen.findByText("You: Waiting")).toBeTruthy();
  });

  it("redirects unknown routes back to the real home screen", async () => {
    renderAppShell("/does-not-exist");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();
  });

  it("keeps host setup blocked until the LAN address resolves", async () => {
    mockedResolveHostLanAddress.mockRejectedValueOnce(
      new Error("No reachable LAN IP"),
    );

    renderAppShell("/host");

    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Host Tournament Setup",
      }),
    ).toBeTruthy();
    expect(await screen.findByText("No reachable LAN IP")).toBeTruthy();
    expect(
      screen
        .getByRole("button", { name: "Continue to lobby" })
        .hasAttribute("disabled"),
    ).toBe(true);
    expect(
      screen.getByText(
        /resolve the lan address before continuing to the lobby/i,
      ),
    ).toBeTruthy();
  });

  it("joins a launch-attached invite straight into the live lobby flow", async () => {
    const bootstrap = createAppBootstrap({
      launchJoinPayload: "pkr1_launch",
      parsedLaunchJoinPayload: createParsedJoinPayload(),
    });

    renderAppShell("/join", bootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Join Tournament" }),
    ).toBeTruthy();
    expect(await screen.findByLabelText("Invite preview")).toBeTruthy();
    expect(
      await screen.findByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();
  });

  it("keeps the lobby route behind an active live session", async () => {
    renderAppShell("/lobby");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();
    expect(
      screen.queryByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeNull();
    expect(screen.queryByText("You: Waiting")).toBeNull();
  });

  it("keeps the table route behind an active live session", async () => {
    renderAppShell("/table", createAppBootstrap(), {
      allowImplicitTableSession: false,
    });

    expect(
      await screen.findByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();
    expect(
      screen.queryByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeNull();
  });

  it("keeps the table route behind an active running session", async () => {
    currentHostSession = {
      ...buildHostSessionStatus({ tournamentName: "Friday Night" }),
      phase: "waitingForPlayers",
    };

    renderAppShell("/table");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();
    expect(
      screen.queryByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeNull();
  });

  it("does not loop back to the table when redirected from a pre-hand table route to the lobby (W1)", async () => {
    // A live session exists but the first hand has not started (pre-hand phase),
    // so /table must redirect to /lobby. The lobby route has no phase-based
    // redirect, so it must NOT bounce back to /table.
    currentHostSession = {
      ...buildHostSessionStatus({ tournamentName: "Friday Night" }),
      phase: "waitingForPlayers",
    };

    renderAppShell("/table", createAppBootstrap(), {
      allowImplicitTableSession: false,
    });

    // The table guard redirects the pre-hand session to the lobby.
    expect(
      await screen.findByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();
    expect(
      screen.queryByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeNull();

    // Let the lobby's own status poll settle, then confirm we are still on the
    // lobby and were never looped back to the table.
    await act(async () => {
      await Promise.resolve();
    });
    expect(
      screen.getByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();
    expect(
      screen.queryByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeNull();
  });

  it("stops the live host session before leaving the lobby", async () => {
    currentHostSession = buildHostSessionStatus({
      tournamentName: "Friday Night",
    });

    renderAppShell("/lobby");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();

    fireEvent.click(await screen.findByRole("button", { name: "Close table" }));
    fireEvent.click(screen.getAllByRole("button", { name: "Close table" })[1]);

    await waitFor(() => {
      expect(mockedStopHostSession).toHaveBeenCalledTimes(1);
    });
    expect(
      await screen.findByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();
  });

  it("leaves the live client session before returning home", async () => {
    currentClientSession = buildClientSessionStatus();

    renderAppShell("/lobby");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();

    expect(await screen.findByText(/awaiting seat/i)).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Leave table" }));
    fireEvent.click(screen.getAllByRole("button", { name: "Leave table" })[1]);

    await waitFor(() => {
      expect(mockedLeaveClientSession).toHaveBeenCalledTimes(1);
    });
    expect(
      await screen.findByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();
  });

  it("reroutes removed ready-room paths back to the home screen", async () => {
    renderAppShell(
      "/ready-room",
      createAppBootstrap({ debugToolsEnabled: true }),
    );

    expect(
      await screen.findByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();
    expect(
      screen.queryByRole("heading", { level: 2, name: "Ready Room" }),
    ).toBeNull();
  });

  it("applies bootstrap subscription updates and reroutes away from routes removed by the new catalog", async () => {
    renderAppShell("/table");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();

    await act(async () => {
      bootstrapSubscriptionHandler?.(
        createAppBootstrap({
          screens: [
            { id: "home", title: "Home", route: "/", surface: "primary" },
            { id: "join", title: "Join", route: "/join", surface: "primary" },
          ],
        }),
      );
    });

    expect(
      await screen.findByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();
    expect(
      screen.queryByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeNull();
    expect(screen.queryByRole("link", { name: "Lobby" })).toBeNull();
  });

  it("recovers a launch-attached invite after a bootstrap update without a full remount", async () => {
    const bootstrap = createAppBootstrap({
      launchJoinPayload: "pkr1_launch",
      launchJoinPayloadError: "Invite signature mismatch",
      parsedLaunchJoinPayload: null,
    });
    localStorage.setItem(
      `${bootstrap.storageNamespace}:join-draft`,
      JSON.stringify("pkr1_launch"),
    );

    renderAppShell("/join", bootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Join Tournament" }),
    ).toBeTruthy();
    expect(await screen.findByText("Invite signature mismatch")).toBeTruthy();
    expect(
      screen.queryByRole("button", { name: "Continue to lobby" }),
    ).toBeTruthy();

    await act(async () => {
      bootstrapSubscriptionHandler?.(
        createAppBootstrap({
          launchJoinPayload: "pkr1_launch",
          launchJoinPayloadError: null,
          parsedLaunchJoinPayload: createParsedJoinPayload(),
        }),
      );
    });

    expect(
      await screen.findByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();
  });

  it("persists shell state and cached hand history across a restart-like remount", async () => {
    const bootstrap = createAppBootstrap({
      storageNamespace: "desktop-poker:restart-flow",
      instanceId: "restart-flow",
      instanceLabel: "Restart Flow",
    });
    mockedGetTableView.mockResolvedValue(
      createTableViewSnapshot({
        phaseLabel: "Complete",
        streetLabel: "Showdown",
        actionOwnerLabel: "Tournament complete",
        actionTray: null,
      }),
    );

    const firstRender = renderAppShell("/join", bootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Join Tournament" }),
    ).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Invite"), {
      target: { value: "pkr1_restart" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Check invite" }));
    expect(await screen.findByLabelText("Invite preview")).toBeTruthy();
    expect(screen.getAllByText("Friday Night").length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "Continue to lobby" }));
    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Lobby",
      }),
    ).toBeTruthy();

    syncLiveSessions(currentClientSession?.participants ?? [], "running");

    firstRender.unmount();

    renderAppShell("/table", bootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
    fireEvent.click(
      await screen.findByRole("link", { name: "Tournament complete" }),
    );

    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Tournament Complete",
      }),
    ).toBeTruthy();
    expect(screen.getByText(/1 hand summaries saved/i)).toBeTruthy();
    expect(
      Object.keys(localStorage).some((key) =>
        key.toLowerCase().includes("reconnect"),
      ),
    ).toBe(false);

    firstRender.unmount();

    mockedGetTableView.mockRejectedValue(new Error("offline"));
    renderAppShell("/history", bootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Hand History" }),
    ).toBeTruthy();
    expect(await screen.findByText("offline")).toBeTruthy();
    expect(screen.getByText(/saved on this device/i)).toBeTruthy();
    expect(
      screen.getAllByText(/host won 210 chip\(s\)\./i).length,
    ).toBeGreaterThan(0);
    expect(
      Object.keys(localStorage).some((key) =>
        key.toLowerCase().includes("reconnect"),
      ),
    ).toBe(false);
  });

  it("keeps table routing while hiding observer preview controls", async () => {
    const firstRender = renderAppShell(
      "/table",
      createAppBootstrap({ debugToolsEnabled: false }),
    );

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
    expect(await screen.findByLabelText("Ace of spades")).toBeTruthy();
    expect(
      screen.queryByRole("button", { name: "Observer preview" }),
    ).toBeNull();
    firstRender.unmount();

    renderAppShell("/table", createAppBootstrap({ debugToolsEnabled: true }));
    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();

    expect(
      screen.queryByRole("button", { name: "Observer preview" }),
    ).toBeNull();

    fireEvent.click(screen.getByRole("link", { name: "Hand history" }));
    expect(
      await screen.findByRole("heading", { level: 2, name: "Hand History" }),
    ).toBeTruthy();
  });

  it("keeps shared public table state aligned while local indicators stay scoped per instance", async () => {
    const aliceBootstrap = createAppBootstrap({
      storageNamespace: "desktop-poker:alice-instance",
      instanceId: "alice-instance",
      instanceLabel: "Alice Instance",
    });
    const bobBootstrap = createAppBootstrap({
      storageNamespace: "desktop-poker:bob-instance",
      instanceId: "bob-instance",
      instanceLabel: "Bob Instance",
    });

    mockedGetTableView.mockReset();
    mockedGetTableView.mockResolvedValue(
      createTableViewSnapshot({
        actionOwnerLabel: "You",
      }),
    );

    const aliceRender = renderAppShell("/table", aliceBootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
    expect(await screen.findByText("Pot 120")).toBeTruthy();
    expect(
      screen.getAllByText(
        (_, element) =>
          element?.textContent?.includes("Player Alice Instance (you)") ??
          false,
      ).length,
    ).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole("button", { name: "Table details" }));
    expect(
      screen.getByText("The flop was published to every seat and observer."),
    ).toBeTruthy();
    expect(screen.getByText("Host won 210 chip(s).")).toBeTruthy();
    expect(await screen.findByRole("button", { name: "Fold" })).toBeTruthy();
    aliceRender.unmount();

    mockedGetTableView.mockResolvedValue(
      createTableViewSnapshot({
        actionOwnerLabel: "Maya",
        seats: [
          {
            ...createTableViewSnapshot().seats[0],
            isLocal: false,
          },
          {
            ...createTableViewSnapshot().seats[1],
            displayName: "Alice",
            isLocal: false,
            isActing: false,
            cardsHidden: true,
            holeCards: [],
          },
          {
            ...createTableViewSnapshot().seats[2],
            displayName: "Bob",
            isLocal: true,
            isActing: true,
            isCompact: false,
            cardsHidden: false,
            holeCards: [
              {
                label: "Queen of spades",
                compactLabel: "Q♠",
                suitSymbol: "♠",
                tone: "dark",
              },
              {
                label: "Jack of hearts",
                compactLabel: "J♥",
                suitSymbol: "♥",
                tone: "red",
              },
            ],
          },
          createTableViewSnapshot().seats[3],
        ],
        standings: [
          {
            ...createTableViewSnapshot().standings[0],
            displayName: "Bob",
            isLocal: true,
          },
          {
            ...createTableViewSnapshot().standings[1],
            displayName: "Alice",
          },
          createTableViewSnapshot().standings[2],
          createTableViewSnapshot().standings[3],
        ],
        actionTray: {
          ...createTableViewSnapshot().actionTray!,
          ownerLabel: "Bob",
        },
      }),
    );

    renderAppShell("/table", bobBootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
    expect(await screen.findByText("Pot 120")).toBeTruthy();
    expect(
      screen.getAllByText(
        (_, element) =>
          element?.textContent?.includes("Player Bob Instance (you)") ?? false,
      ).length,
    ).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole("button", { name: "Table details" }));
    expect(
      screen.getByText("The flop was published to every seat and observer."),
    ).toBeTruthy();
    expect(screen.getByText("Host won 210 chip(s).")).toBeTruthy();
    expect(screen.getByText("Maya")).toBeTruthy();
    expect(await screen.findByRole("button", { name: "Fold" })).toBeTruthy();
  });

  it("keeps the live table snapshot stable when an action fails and recovers on the next successful retry", async () => {
    mockedGetTableView.mockResolvedValue(createTableViewSnapshot());
    mockedSubmitTableAction
      .mockRejectedValueOnce(new Error("Action window expired."))
      .mockResolvedValueOnce(
        createTableViewSnapshot({
          streetLabel: "Turn",
          potTotal: 240,
          actionOwnerLabel: "Maya",
          eventFeed: [
            {
              sequence: 19,
              kind: "public-event",
              message:
                "Maya called and the turn was published to every seat and observer.",
            },
          ],
          handHistory: [
            {
              handNumber: 9,
              summary: "Maya won 240 chip(s).",
              potTotal: 240,
              winningPlayers: ["Maya"],
              eliminatedPlayers: [],
              boardCards: [],
            },
            ...createTableViewSnapshot().handHistory,
          ],
          actionTray: null,
        }),
      );

    renderAppShell("/table");

    expect(await screen.findByRole("button", { name: "Fold" })).toBeTruthy();
    expect((await screen.findAllByText(/Pot 120/)).length).toBeGreaterThan(0);
    expect(await screen.findByLabelText("Ace of spades")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Fold" }));

    expect(await screen.findByText("Action window expired.")).toBeTruthy();
    expect(screen.getByText("Pot 120")).toBeTruthy();
    expect(screen.getByLabelText("Ace of spades")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Check" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Check" }));

    expect((await screen.findAllByText(/Pot 240/)).length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole("button", { name: "Table details" }));
    expect(
      screen.getByText(
        /maya called and the turn was published to every seat and observer/i,
      ),
    ).toBeTruthy();
    expect(screen.queryByText("Action window expired.")).toBeNull();
  });

  it("shows a truthful waiting table state when no local action window is open", async () => {
    mockedGetTableView.mockResolvedValue(
      createTableViewSnapshot({
        actionOwnerLabel: "Maya",
        actionTray: null,
      }),
    );

    const tableRender = renderAppShell("/table");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Fold" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Check" })).toBeNull();
    expect(screen.queryByRole("button", { name: /bet \/ raise/i })).toBeNull();
    expect(screen.queryByRole("button", { name: "All-in" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Table details" }));
    expect(screen.getAllByText("Maya").length).toBeGreaterThan(0);

    tableRender.unmount();
  });

  it("keeps action failures scoped to the acting shell instance", async () => {
    const aliceBootstrap = createAppBootstrap({
      storageNamespace: "desktop-poker:acting-shell-a",
      instanceId: "acting-shell-a",
      instanceLabel: "Acting Shell A",
    });
    const bobBootstrap = createAppBootstrap({
      storageNamespace: "desktop-poker:acting-shell-b",
      instanceId: "acting-shell-b",
      instanceLabel: "Acting Shell B",
    });

    mockedGetTableView.mockResolvedValue(createTableViewSnapshot());
    mockedSubmitTableAction.mockRejectedValueOnce(
      new Error("Action window expired."),
    );

    const aliceRender = renderAppShell("/table", aliceBootstrap);

    fireEvent.click(await screen.findByRole("button", { name: "Fold" }));
    expect(await screen.findByText("Action window expired.")).toBeTruthy();
    aliceRender.unmount();

    renderAppShell("/table", bobBootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
    expect(screen.queryByText("Action window expired.")).toBeNull();
    expect((await screen.findAllByText(/Pot 120/)).length).toBeGreaterThan(0);
    expect(
      screen.getAllByText(/player acting shell b \(you\)/i).length,
    ).toBeGreaterThan(0);
  });

  it("ignores reconnect metadata at boot when a live table snapshot is still available", async () => {
    const bootstrap = createAppBootstrap({
      storageNamespace: "desktop-poker:reconnect-live",
      instanceId: "reconnect-live",
      instanceLabel: "Reconnect Live",
      reconnectNamespace: "desktop-reconnect:reconnect-live",
    });
    localStorage.setItem(
      bootstrap.reconnectNamespace,
      JSON.stringify({ tableId: "old-table", token: "stale-token" }),
    );

    mockedGetTableView.mockResolvedValue(createTableViewSnapshot());

    renderAppShell("/table", bootstrap);

    expect(await screen.findByRole("button", { name: "Fold" })).toBeTruthy();
    expect(screen.queryByText(/reconnecting to the table/i)).toBeNull();
    expect(localStorage.getItem(bootstrap.reconnectNamespace)).toContain(
      "stale-token",
    );
  });

  it("ignores stale reconnect metadata and falls back to the normal table-unavailable surface", async () => {
    const bootstrap = createAppBootstrap({
      storageNamespace: "desktop-poker:reconnect-stale",
      instanceId: "reconnect-stale",
      instanceLabel: "Reconnect Stale",
      reconnectNamespace: "desktop-reconnect:reconnect-stale",
    });
    localStorage.setItem(
      bootstrap.reconnectNamespace,
      JSON.stringify({ tableId: "old-table", token: "stale-token" }),
    );

    mockedGetTableView.mockRejectedValue(
      new Error("Host connection lost. Reopen the lobby or rejoin."),
    );

    renderAppShell("/table", bootstrap);

    expect(
      await screen.findByText(
        "Host connection lost. Reopen the lobby or rejoin.",
      ),
    ).toBeTruthy();
    expect(screen.getByText("Table unavailable")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Return to lobby" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Open history" })).toBeTruthy();
    expect(screen.queryByText(/reconnecting to the table/i)).toBeNull();
  });

  it("keeps reconnect metadata isolated per instance and leaves history restarts on the cached-history surface", async () => {
    const firstBootstrap = createAppBootstrap({
      storageNamespace: "desktop-poker:reconnect-a",
      instanceId: "reconnect-a",
      instanceLabel: "Reconnect A",
      reconnectNamespace: "desktop-reconnect:reconnect-a",
    });
    const secondBootstrap = createAppBootstrap({
      storageNamespace: "desktop-poker:reconnect-b",
      instanceId: "reconnect-b",
      instanceLabel: "Reconnect B",
      reconnectNamespace: "desktop-reconnect:reconnect-b",
    });

    localStorage.setItem(
      firstBootstrap.reconnectNamespace,
      JSON.stringify({ tableId: "first-table", token: "first-token" }),
    );
    localStorage.setItem(
      secondBootstrap.reconnectNamespace,
      JSON.stringify({ tableId: "second-table", token: "second-token" }),
    );
    localStorage.setItem(
      storageKey(secondBootstrap.storageNamespace, "hand-history-summaries"),
      JSON.stringify({
        updatedAtMs: 1,
        entries: [
          {
            handNumber: 14,
            summary: "Reconnect B won 300 chip(s).",
            potTotal: 300,
            winningPlayers: ["Reconnect B"],
            eliminatedPlayers: [],
            boardCards: [],
          },
        ],
      }),
    );

    mockedGetTableView.mockRejectedValue(new Error("offline"));

    renderAppShell("/history", secondBootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Hand History" }),
    ).toBeTruthy();
    expect(await screen.findByText("offline")).toBeTruthy();
    expect(screen.getByText(/saved on this device/i)).toBeTruthy();
    expect(screen.getByText(/reconnect b won 300 chip\(s\)\./i)).toBeTruthy();
    expect(screen.queryByText(/reconnecting to the table/i)).toBeNull();
    expect(localStorage.getItem(firstBootstrap.reconnectNamespace)).toContain(
      "first-token",
    );
    expect(localStorage.getItem(secondBootstrap.reconnectNamespace)).toContain(
      "second-token",
    );
  });

  it("enters the real debug surface and reflects a payload-free child launch without corrupting the current shell state", async () => {
    const bootstrap = createAppBootstrap({
      debugToolsEnabled: true,
      storageNamespace: "desktop-poker:debug-parent",
      instanceId: "debug-parent",
      instanceLabel: "Debug Parent",
      launchJoinPayload: null,
    });
    localStorage.setItem(
      storageKey(bootstrap.storageNamespace, "host-draft"),
      JSON.stringify({
        tournamentName: "Night Debug",
        maxPlayers: 6,
        startingStack: 1500,
        blindPresetId: "normal",
        turnTimerSeconds: 30,
        hostPort: 43818,
      }),
    );

    renderAppShell("/debug", bootstrap);

    expect(await screen.findByText("Debug Parent")).toBeTruthy();
    expect(await screen.findByText(/Spawn another debug client/i)).toBeTruthy();
    expect(
      screen.getAllByText(
        (_, element) =>
          element?.textContent?.includes("Host draft: Night Debug") ?? false,
      ).length,
    ).toBeGreaterThan(0);
    expect(
      screen.getAllByText(
        (_, element) =>
          element?.textContent?.includes("Current sequence: 17") ?? false,
      ).length,
    ).toBeGreaterThan(0);

    fireEvent.click(
      screen.getByRole("button", { name: "Launch extra client" }),
    );

    expect(await screen.findByText("Launched debug-child-1")).toBeTruthy();
    expect(mockedLaunchAdditionalClientInstance).toHaveBeenCalledWith(null);
    expect(clipboardWriteText).not.toHaveBeenCalled();
    expect(
      screen.getByRole("button", { name: "Launch extra client" }),
    ).toBeTruthy();
    expect(
      screen.getAllByText(
        (_, element) =>
          element?.textContent?.includes("Host draft: Night Debug") ?? false,
      ).length,
    ).toBeGreaterThan(0);
  });

  it("keeps a payload-attached debug launch on the current debug route and state", async () => {
    const bootstrap = createAppBootstrap({
      debugToolsEnabled: true,
      storageNamespace: "desktop-poker:debug-payload",
      instanceId: "debug-payload",
      instanceLabel: "Debug Payload",
      launchJoinPayload: "pkr1_attached_payload",
    });

    renderAppShell("/debug", bootstrap);

    expect(
      await screen.findByDisplayValue("pkr1_attached_payload"),
    ).toBeTruthy();
    expect(await screen.findByText(/legal Fold, Check, Bet/i)).toBeTruthy();

    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", {
          name: "Launch extra client with payload",
        }),
      );
    });

    expect(
      await screen.findByText(
        "Launched debug-child-1 with the copied join payload attached.",
      ),
    ).toBeTruthy();
    expect(clipboardWriteText).toHaveBeenCalledWith("pkr1_attached_payload");
    expect(mockedLaunchAdditionalClientInstance).toHaveBeenCalledWith(
      "pkr1_attached_payload",
    );
    expect(
      screen.getAllByText(
        (_, element) =>
          element?.textContent?.includes("Current sequence: 17") ?? false,
      ).length,
    ).toBeGreaterThan(0);
    expect(
      screen.getByRole("button", { name: "Launch extra client with payload" }),
    ).toBeTruthy();
  });

  it("keeps launch-driven namespace continuity isolated across parent and child shell instances", async () => {
    const parentBootstrap = createAppBootstrap({
      debugToolsEnabled: true,
      storageNamespace: "desktop-poker:launch-parent",
      instanceId: "launch-parent",
      instanceLabel: "Launch Parent",
    });
    const childBootstrap = createAppBootstrap({
      debugToolsEnabled: true,
      storageNamespace: "desktop-poker:launch-child",
      instanceId: "launch-child",
      instanceLabel: "Launch Child",
    });

    localStorage.setItem(
      storageKey(parentBootstrap.storageNamespace, "host-draft"),
      JSON.stringify({
        tournamentName: "Parent Room",
        maxPlayers: 6,
        startingStack: 1500,
        blindPresetId: "normal",
        turnTimerSeconds: 30,
        hostPort: 43818,
      }),
    );
    localStorage.setItem(
      storageKey(parentBootstrap.storageNamespace, "ready-seats"),
      JSON.stringify([2]),
    );
    localStorage.setItem(
      storageKey(childBootstrap.storageNamespace, "host-draft"),
      JSON.stringify({
        tournamentName: "Child Room",
        maxPlayers: 6,
        startingStack: 1500,
        blindPresetId: "normal",
        turnTimerSeconds: 30,
        hostPort: 43818,
      }),
    );
    localStorage.setItem(
      storageKey(childBootstrap.storageNamespace, "ready-seats"),
      JSON.stringify([1]),
    );
    localStorage.setItem(
      storageKey(childBootstrap.storageNamespace, "hand-history-summaries"),
      JSON.stringify({
        updatedAtMs: 1,
        entries: [
          {
            handNumber: 22,
            summary: "Launch Child won 440 chip(s).",
            potTotal: 440,
            winningPlayers: ["Launch Child"],
            eliminatedPlayers: [],
            boardCards: [],
          },
        ],
      }),
    );

    const parentRender = renderAppShell("/debug", parentBootstrap);

    expect(await screen.findByText("Launch Parent")).toBeTruthy();
    fireEvent.click(
      screen.getByRole("button", { name: "Launch extra client" }),
    );
    expect(await screen.findByText("Launched debug-child-1")).toBeTruthy();
    expect(
      localStorage.getItem(
        storageKey(parentBootstrap.storageNamespace, "ready-seats"),
      ),
    ).toBe("[2]");
    expect(
      localStorage.getItem(
        storageKey(childBootstrap.storageNamespace, "ready-seats"),
      ),
    ).toBe("[1]");
    parentRender.unmount();

    const childDebugRender = renderAppShell("/debug", childBootstrap);

    expect(await screen.findByText("Launch Child")).toBeTruthy();
    expect(await screen.findByText(/Spawn another debug client/i)).toBeTruthy();
    expect(
      screen.getAllByText(
        (_, element) =>
          element?.textContent?.includes("Host draft: Child Room") ?? false,
      ).length,
    ).toBeGreaterThan(0);
    expect(screen.queryByText("Parent Room")).toBeNull();
    childDebugRender.unmount();

    mockedGetTableView.mockRejectedValueOnce(new Error("offline"));

    renderAppShell("/history", childBootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Hand History" }),
    ).toBeTruthy();
    expect(screen.getByText(/launch child won 440 chip\(s\)\./i)).toBeTruthy();
    expect(screen.queryByText(/parent room/i)).toBeNull();
  });

  it("keeps join and table failures inside explicit shell states instead of stranding navigation", async () => {
    mockedValidateJoinPayloadInput.mockRejectedValueOnce(
      new Error("Join payload rejected"),
    );

    renderAppShell("/join");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Join Tournament" }),
    ).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Invite"), {
      target: { value: "pkr1_bad" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Check invite" }));

    expect(await screen.findByText("Join payload rejected")).toBeTruthy();
    expect(
      screen
        .getByRole("button", { name: "Continue to lobby" })
        .hasAttribute("disabled"),
    ).toBe(true);

    mockedGetTableView.mockRejectedValue(
      new Error("Host connection lost. Reopen the lobby or rejoin."),
    );

    renderAppShell("/table");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
    expect(
      await screen.findByText(
        "Host connection lost. Reopen the lobby or rejoin.",
      ),
    ).toBeTruthy();

    fireEvent.click(screen.getByRole("link", { name: "Hand history" }));
    expect(
      await screen.findByRole("heading", { level: 2, name: "Hand History" }),
    ).toBeTruthy();
  });

  it("transitions an eliminated player into observer-only state after a live table action", async () => {
    mockedGetTableView.mockResolvedValue(createTableViewSnapshot());
    mockedSubmitTableAction.mockResolvedValueOnce(
      createTableViewSnapshot({
        actionOwnerLabel: "Maya",
        observerBanner:
          "You have been eliminated and now remain connected as a public-only observer.",
        eliminationSummary:
          "You busted on hand 9 and now remain at the table as a public-only observer.",
        seats: [
          createTableViewSnapshot().seats[0],
          {
            ...createTableViewSnapshot().seats[1],
            statusLabel: "Eliminated observer",
            markerLabel: "Observer",
            chipCount: 0,
            contribution: 0,
            isActing: false,
            isObserver: true,
            isEliminated: true,
            cardsHidden: true,
            holeCards: [],
            detailLines: ["Busted on hand 9 after finishing 3rd."],
          },
          {
            ...createTableViewSnapshot().seats[2],
            isActing: true,
          },
          createTableViewSnapshot().seats[3],
        ],
        standings: [
          {
            ...createTableViewSnapshot().standings[0],
            chipCount: 0,
            statusLabel: "Eliminated observer",
            note: "Busted on hand 9",
            isObserver: true,
          },
          createTableViewSnapshot().standings[1],
          createTableViewSnapshot().standings[2],
          createTableViewSnapshot().standings[3],
        ],
        handHistory: [
          {
            handNumber: 9,
            summary: "Maya won 240 chip(s).",
            potTotal: 240,
            winningPlayers: ["Maya"],
            eliminatedPlayers: ["You"],
            boardCards: [],
          },
          ...createTableViewSnapshot().handHistory,
        ],
        actionTray: null,
      }),
    );

    renderAppShell("/table");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
    fireEvent.click(await screen.findByRole("button", { name: "Fold" }));

    expect(
      await screen.findByText(
        /you have been eliminated and now remain connected as a public-only observer/i,
      ),
    ).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Fold" })).toBeNull();
    expect(screen.queryByLabelText("Ace of spades")).toBeNull();
    expect(screen.getByText(/you busted on hand 9/i)).toBeTruthy();

    fireEvent.click(screen.getByRole("link", { name: "Hand history" }));
    expect(await screen.findByText(/maya won 240 chip\(s\)\./i)).toBeTruthy();
  });

  it("keeps the desktop shell fixed to the viewport so all primary UI controls remain reachable", async () => {
    renderAppShell("/");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();

    const appFrame = document.querySelector(".app-frame");
    expect(appFrame).toBeTruthy();
    expect(appFrame).toBeInstanceOf(HTMLElement);
    expect((appFrame as HTMLElement).style.height).toBe("100dvh");
    expect((appFrame as HTMLElement).style.overflow).toBe("hidden");

    const content = appFrame?.querySelector(".content");
    expect(content).toBeTruthy();
    expect(content).toBeInstanceOf(HTMLElement);
    expect((content as HTMLElement).style.height).toBe("100%");
    expect((content as HTMLElement).style.overflow).toBe("hidden");

    expect(screen.getByRole("link", { name: "Host Tournament" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Join Tournament" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Help" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Settings" })).toBeTruthy();
  });
});
