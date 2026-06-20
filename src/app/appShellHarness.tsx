import { render } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { vi } from "vitest";
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
import { DesktopBootstrapProvider } from "./DesktopBootstrapProvider";
import { AppShell } from "./AppShell";

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

type AppShellCtx = {
  currentHostSession: Awaited<ReturnType<typeof startHostSession>> | null;
  currentClientSession: Awaited<ReturnType<typeof joinHostSession>> | null;
  bootstrapSubscriptionHandler:
    | ((bootstrap: ReturnType<typeof createAppBootstrap>) => void)
    | undefined;
};

const ctx: AppShellCtx = {
  currentHostSession: null,
  currentClientSession: null,
  bootstrapSubscriptionHandler: undefined,
};

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
    tournamentName: ctx.currentHostSession?.tournamentName ?? "Friday Night",
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
    terminated: false,
    lastError: null,
    participants: [
      {
        playerId: "local-player",
        displayName:
          ctx.currentHostSession?.participants[0]?.displayName ?? "Host Alpha",
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
  participants: NonNullable<typeof ctx.currentHostSession>["participants"],
  forcedPhase?: string,
) {
  const totalSeats = ctx.currentHostSession
    ? ctx.currentHostSession.activeSeatCount +
      ctx.currentHostSession.openSeatCount
    : ctx.currentClientSession
      ? ctx.currentClientSession.activeSeatCount +
        ctx.currentClientSession.openSeatCount
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

  if (ctx.currentHostSession) {
    ctx.currentHostSession = {
      ...ctx.currentHostSession,
      participants,
      activeSeatCount,
      openSeatCount: totalSeats - activeSeatCount,
      phase,
    };
  }

  if (ctx.currentClientSession) {
    ctx.currentClientSession = {
      ...ctx.currentClientSession,
      tournamentName:
        ctx.currentHostSession?.tournamentName ??
        ctx.currentClientSession.tournamentName,
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
  // allowImplicitTableSession: tests that navigate directly to /table without
  // setting up a session first must opt in explicitly. Defaults to false so
  // new tests cannot accidentally rely on the implicit fallback.
  options?: { allowImplicitTableSession?: boolean },
) {
  if (
    options?.allowImplicitTableSession === true &&
    initialEntry === "/table" &&
    !ctx.currentHostSession &&
    !ctx.currentClientSession
  ) {
    ctx.currentHostSession = {
      ...buildHostSessionStatus({
        tournamentName: "Friday Night",
      }),
      phase: "running",
    };
  }

  mockedFetchBootstrapState.mockResolvedValue(bootstrap);
  mockedSubscribeBootstrap.mockImplementation(async (onBootstrap) => {
    ctx.bootstrapSubscriptionHandler = onBootstrap;
    return () => {
      ctx.bootstrapSubscriptionHandler = undefined;
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

function setupAppShellMocks() {
  localStorage.clear();
  ctx.bootstrapSubscriptionHandler = undefined;
  ctx.currentHostSession = null;
  ctx.currentClientSession = null;
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
    async () => ctx.currentClientSession,
  );
  mockedGetHostSessionStatus.mockImplementation(
    async () => ctx.currentHostSession,
  );
  mockedStartHostSession.mockImplementation(async (request) => {
    ctx.currentHostSession = buildHostSessionStatus({
      tournamentName: request.tournamentName,
      maxPlayers: request.maxPlayers,
      displayName: request.displayName,
    });
    return ctx.currentHostSession;
  });
  mockedAddNpcPlayers.mockImplementation(async ({ npcs }) => {
    if (!ctx.currentHostSession) {
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
      ...ctx.currentHostSession.participants,
      ...npcParticipants,
    ]);
    return ctx.currentHostSession as NonNullable<typeof ctx.currentHostSession>;
  });
  mockedJoinHostSession.mockImplementation(async () => {
    ctx.currentClientSession = buildClientSessionStatus();
    if (ctx.currentHostSession) {
      syncLiveSessions(ctx.currentClientSession.participants);
    }
    return ctx.currentClientSession!;
  });
  mockedHostClaimLobbySeat.mockImplementation(async (request) => {
    if (!ctx.currentHostSession) {
      throw new Error("No active host session");
    }

    const nextParticipants = ctx.currentHostSession.participants.map(
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
    return ctx.currentHostSession;
  });
  mockedHostSetLobbyReadyState.mockImplementation(async (request) => {
    if (!ctx.currentHostSession) {
      throw new Error("No active host session");
    }

    const nextParticipants = ctx.currentHostSession.participants.map(
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
    return ctx.currentHostSession;
  });
  mockedHostStartTournament.mockImplementation(async () => {
    if (!ctx.currentHostSession) {
      throw new Error("No active host session");
    }

    syncLiveSessions(ctx.currentHostSession.participants, "running");
    return ctx.currentHostSession;
  });
  mockedStopHostSession.mockImplementation(async () => {
    ctx.currentHostSession = null;
  });
  mockedLeaveClientSession.mockImplementation(async () => {
    ctx.currentClientSession = null;
  });
  mockedClientClaimLobbySeat.mockImplementation(async (request) => {
    if (!ctx.currentClientSession) {
      throw new Error("No active client session");
    }

    const nextParticipants = ctx.currentClientSession.participants.map(
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
    return ctx.currentClientSession;
  });
  mockedClientSetLobbyReadyState.mockImplementation(async (request) => {
    if (!ctx.currentClientSession) {
      throw new Error("No active client session");
    }

    const nextParticipants = ctx.currentClientSession.participants.map(
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
    return ctx.currentClientSession;
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
    lastLlmFallback: null,
    lastNpcActionError: null,
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
        viewerMode === "observer" ? null : createTableViewSnapshot().actionTray,
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
}

export const h = {
  ctx,
  renderAppShell,
  syncLiveSessions,
  setupAppShellMocks,
  buildHostSessionStatus,
  buildClientSessionStatus,
  clipboardWriteText,
  mockedFetchBootstrapState,
  mockedAddNpcPlayers,
  mockedListNpcProfiles,
  mockedClientClaimLobbySeat,
  mockedClientSetLobbyReadyState,
  mockedGetClientSessionStatus,
  mockedGetHostSessionStatus,
  mockedSubscribeBootstrap,
  mockedGetDebugState,
  mockedHostClaimLobbySeat,
  mockedLeaveClientSession,
  mockedHostSetLobbyReadyState,
  mockedHostStartTournament,
  mockedJoinHostSession,
  mockedLaunchAdditionalClientInstance,
  mockedOnSessionUpdate,
  mockedOnTableUpdate,
  mockedResolveHostLanAddress,
  mockedStopHostSession,
  mockedStartHostSession,
  mockedValidateJoinPayloadInput,
  mockedGetTableView,
  mockedSubmitTableAction,
};
