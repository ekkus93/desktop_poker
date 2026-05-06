import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getClientSessionStatus,
  fetchBootstrapState,
  getHostSessionStatus,
  getDebugState,
  getTableView,
  joinHostSession,
  launchAdditionalClientInstance,
  resolveHostLanAddress,
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
  const actual = await vi.importActual<typeof import("../api/desktop")>(
    "../api/desktop",
  );

  return {
    ...actual,
    fetchBootstrapState: vi.fn(),
    getClientSessionStatus: vi.fn(),
    getHostSessionStatus: vi.fn(),
    subscribeBootstrap: vi.fn(),
    getDebugState: vi.fn(),
    joinHostSession: vi.fn(),
    launchAdditionalClientInstance: vi.fn(),
    resolveHostLanAddress: vi.fn(),
    startHostSession: vi.fn(),
    validateJoinPayloadInput: vi.fn(),
    getTableView: vi.fn(),
    submitTableAction: vi.fn(),
  };
});

const mockedFetchBootstrapState = vi.mocked(fetchBootstrapState);
const mockedGetClientSessionStatus = vi.mocked(getClientSessionStatus);
const mockedGetHostSessionStatus = vi.mocked(getHostSessionStatus);
const mockedSubscribeBootstrap = vi.mocked(subscribeBootstrap);
const mockedGetDebugState = vi.mocked(getDebugState);
const mockedJoinHostSession = vi.mocked(joinHostSession);
const mockedLaunchAdditionalClientInstance = vi.mocked(launchAdditionalClientInstance);
const mockedResolveHostLanAddress = vi.mocked(resolveHostLanAddress);
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
        displayName: currentHostSession?.participants[0]?.displayName ?? "Host Alpha",
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

function renderAppShell(initialEntry: string, bootstrap = createAppBootstrap()) {
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
    mockedGetClientSessionStatus.mockReset();
    mockedGetHostSessionStatus.mockReset();
    mockedSubscribeBootstrap.mockReset();
    mockedGetDebugState.mockReset();
    mockedJoinHostSession.mockReset();
    mockedLaunchAdditionalClientInstance.mockReset();
    mockedResolveHostLanAddress.mockReset();
    mockedStartHostSession.mockReset();
    mockedValidateJoinPayloadInput.mockReset();
    mockedGetTableView.mockReset();
    mockedSubmitTableAction.mockReset();
    clipboardWriteText.mockReset();
    mockedGetClientSessionStatus.mockImplementation(async () => currentClientSession);
    mockedGetHostSessionStatus.mockImplementation(async () => currentHostSession);
    mockedStartHostSession.mockImplementation(async (request) => {
      currentHostSession = buildHostSessionStatus({
        tournamentName: request.tournamentName,
        maxPlayers: request.maxPlayers,
        displayName: request.displayName,
      });
      return currentHostSession;
    });
    mockedJoinHostSession.mockImplementation(async () => {
      currentClientSession = buildClientSessionStatus();
      if (currentHostSession) {
        currentHostSession = {
          ...currentHostSession,
          participants: currentClientSession.participants,
        };
      }
      return currentClientSession;
    });
    mockedResolveHostLanAddress.mockResolvedValue("192.168.1.10");
    mockedValidateJoinPayloadInput.mockResolvedValue(createParsedJoinPayload());
    mockedGetDebugState.mockResolvedValue({
      protocolLog: [],
      snapshotJson: "{}",
      currentSequence: 17,
      currentHandNumber: 9,
      actionWindowSummary: "You · check or bet · min 60 · max 1520 · legal Fold, Check, Bet",
      launchHint:
        "Spawn another debug client with its own storage namespace, or attach a copied pkr1_ payload to exercise local multi-instance join handoff.",
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

    expect(screen.getByRole("region", { name: /host share summary/i })).toBeTruthy();
    expect(screen.getByText("Friday Finals")).toBeTruthy();
    expect(screen.getByText(/192\.168\.1\.10:43818/i)).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Start hosting" }));
    await waitFor(() => {
      expect(mockedStartHostSession).toHaveBeenCalled();
    });
    fireEvent.click(
      screen.getByRole("link", { name: "Continue to lobby" }),
    );

    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Lobby",
      }),
    ).toBeTruthy();
    expect(screen.getByText("Friday Finals")).toBeTruthy();
    expect(screen.getByText("Seat 1")).toBeTruthy();
    expect(screen.getByText("Seat 6")).toBeTruthy();
    expect(screen.getAllByText("Friday Finals").length).toBeGreaterThan(0);
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
    fireEvent.click(
      screen.getByRole("button", { name: "Continue to lobby" }),
    );

    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Lobby",
      }),
    ).toBeTruthy();

    expect(screen.getByRole("button", { name: "Start tournament" }).hasAttribute("disabled")).toBe(true);
    await waitFor(() => {
      expect(screen.queryByRole("button", { name: "I'm ready" })).toBeNull();
    });
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
    await screen.findByText(/live on 192\.168\.1\.10/i);
    fireEvent.click(screen.getByRole("button", { name: "Copy invite" }));

    await waitFor(() => {
      expect(mockedStartHostSession).toHaveBeenCalledWith({
        hostAddress: "192.168.1.10",
        hostPort: 43818,
        tournamentName: "Invite Finals",
        maxPlayers: 6,
        startingStack: 1500,
        blindPresetId: "standard",
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
    expect(screen.getByText("You: Waiting")).toBeTruthy();
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
    expect(screen.getByRole("button", { name: "Continue to lobby" }).hasAttribute("disabled")).toBe(true);
    expect(screen.getByText(/resolve the lan address before continuing to the lobby/i)).toBeTruthy();
  });

  it("lets a launch-attached invite continue straight into the lobby flow", async () => {
    const bootstrap = createAppBootstrap({
      launchJoinPayload: "pkr1_launch",
      parsedLaunchJoinPayload: createParsedJoinPayload(),
    });

    renderAppShell("/join", bootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Join Tournament" }),
    ).toBeTruthy();
    expect(await screen.findByLabelText("Invite preview")).toBeTruthy();
    expect(screen.getByText(/invite already attached to this launch/i)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Continue to lobby" })).toBeTruthy();
  });

  it("keeps the lobby readiness badge aligned with the local seat state", async () => {
    renderAppShell("/lobby");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();
    expect(screen.getByText("You: Waiting")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "I'm ready" }));

    expect(await screen.findByText("You: Ready")).toBeTruthy();
  });

  it("renders the real ready-room route and only unlocks start after all visible participants are ready", async () => {
    renderAppShell("/ready-room", createAppBootstrap({ debugToolsEnabled: true }));

    expect(
      await screen.findByRole("heading", { level: 2, name: "Ready Room" }),
    ).toBeTruthy();
    expect(screen.getByText("Turn timer 30s")).toBeTruthy();
    expect(screen.getByText("2 participants")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Start tournament" }).hasAttribute("disabled")).toBe(true);
    expect(screen.getByRole("button", { name: "Not ready" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Host marks ready" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Not ready" }));
    fireEvent.click(screen.getByRole("button", { name: "Host marks ready" }));

    expect(screen.queryByRole("button", { name: "Start tournament" })).toBeNull();
    expect(screen.getByRole("link", { name: "Start tournament" }).getAttribute("href")).toBe("/table");
  });

  it("keeps remote readiness passive outside debug mode and preserves state across the ready-room leave flow", async () => {
    renderAppShell("/ready-room", createAppBootstrap({ debugToolsEnabled: false }));

    expect(
      await screen.findByRole("heading", { level: 2, name: "Ready Room" }),
    ).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Host marks ready" })).toBeNull();
    expect(screen.getByText("Waiting on player readiness.")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Not ready" }));
    expect(screen.getByRole("button", { name: "Ready" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Leave table" }));
    expect(screen.getByText("Leave before start?")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Stay ready" }));
    expect(screen.queryByText("Leave before start?")).toBeNull();
    expect(screen.getByRole("button", { name: "Ready" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Leave table" }));
    fireEvent.click(screen.getByRole("link", { name: "Leave table" }));

    expect(
      await screen.findByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();
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
    expect(screen.queryByRole("heading", { level: 2, name: "Main Table" })).toBeNull();
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
    expect(screen.queryByRole("button", { name: "Continue to lobby" })).toBeTruthy();

    await act(async () => {
      bootstrapSubscriptionHandler?.(
        createAppBootstrap({
          launchJoinPayload: "pkr1_launch",
          launchJoinPayloadError: null,
          parsedLaunchJoinPayload: createParsedJoinPayload(),
        }),
      );
    });

    expect(await screen.findByText(/invite already attached to this launch/i)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Continue to lobby" })).toBeTruthy();
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

    fireEvent.click(
      screen.getByRole("button", { name: "Continue to lobby" }),
    );
    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Lobby",
      }),
    ).toBeTruthy();

    firstRender.unmount();

    renderAppShell("/table", bootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
    fireEvent.click(await screen.findByRole("link", { name: "Tournament complete" }));

    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Tournament Complete",
      }),
    ).toBeTruthy();
    expect(screen.getByText(/1 hand summaries saved/i)).toBeTruthy();
    expect(
      Object.keys(localStorage).some((key) => key.toLowerCase().includes("reconnect")),
    ).toBe(false);

    firstRender.unmount();

    mockedGetTableView.mockRejectedValue(new Error("offline"));
    renderAppShell("/history", bootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Hand History" }),
    ).toBeTruthy();
    expect(await screen.findByText("offline")).toBeTruthy();
    expect(
      screen.getByText(/saved on this device/i),
    ).toBeTruthy();
    expect(screen.getAllByText(/host won 210 chip\(s\)\./i).length).toBeGreaterThan(0);
    expect(
      Object.keys(localStorage).some((key) => key.toLowerCase().includes("reconnect")),
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
    expect(screen.queryByRole("button", { name: "Observer preview" })).toBeNull();
    firstRender.unmount();

    renderAppShell("/table", createAppBootstrap({ debugToolsEnabled: true }));
    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();

    expect(screen.queryByRole("button", { name: "Observer preview" })).toBeNull();

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
    mockedGetTableView.mockResolvedValueOnce(
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
          element?.textContent?.includes("Player Alice Instance (you)") ?? false,
      ).length,
    ).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole("button", { name: "Table details" }));
    expect(screen.getByText("The flop was published to every seat and observer.")).toBeTruthy();
    expect(screen.getByText("Host won 210 chip(s).")).toBeTruthy();
    expect(await screen.findByRole("button", { name: "Fold" })).toBeTruthy();
    aliceRender.unmount();

    mockedGetTableView.mockResolvedValueOnce(
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
    expect(screen.getByText("The flop was published to every seat and observer.")).toBeTruthy();
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
              message: "Maya called and the turn was published to every seat and observer.",
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
    expect(screen.getByText(/maya called and the turn was published to every seat and observer/i)).toBeTruthy();
    expect(screen.queryByText("Action window expired.")).toBeNull();
  });

  it("shows a truthful waiting table state when no local action window is open", async () => {
    mockedGetTableView.mockResolvedValue(
      createTableViewSnapshot({
        actionOwnerLabel: "Maya",
        actionTray: null,
      }),
    );

    renderAppShell("/table");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Fold" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Check" })).toBeNull();
    expect(screen.queryByRole("button", { name: /bet \/ raise/i })).toBeNull();
    expect(screen.queryByRole("button", { name: "All-in" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Table details" }));
    expect(screen.getAllByText("Maya").length).toBeGreaterThan(0);
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
    mockedSubmitTableAction.mockRejectedValueOnce(new Error("Action window expired."));

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
    expect(screen.getAllByText(/player acting shell b \(you\)/i).length).toBeGreaterThan(0);
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
    expect(localStorage.getItem(bootstrap.reconnectNamespace)).toContain("stale-token");
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

    mockedGetTableView.mockRejectedValueOnce(
      new Error("Host connection lost. Reopen the lobby or rejoin."),
    );

    renderAppShell("/table", bootstrap);

    expect(
      await screen.findByText("Host connection lost. Reopen the lobby or rejoin."),
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

    mockedGetTableView.mockRejectedValueOnce(new Error("offline"));

    renderAppShell("/history", secondBootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Hand History" }),
    ).toBeTruthy();
    expect(await screen.findByText("offline")).toBeTruthy();
    expect(screen.getByText(/saved on this device/i)).toBeTruthy();
    expect(screen.getByText(/reconnect b won 300 chip\(s\)\./i)).toBeTruthy();
    expect(screen.queryByText(/reconnecting to the table/i)).toBeNull();
    expect(localStorage.getItem(firstBootstrap.reconnectNamespace)).toContain("first-token");
    expect(localStorage.getItem(secondBootstrap.reconnectNamespace)).toContain("second-token");
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
        blindPresetId: "standard",
        turnTimerSeconds: 30,
        hostPort: 43818,
      }),
    );

    renderAppShell("/debug", bootstrap);

    expect(await screen.findByText("Debug Parent")).toBeTruthy();
    expect(await screen.findByText(/Spawn another debug client/i)).toBeTruthy();
    expect(
      screen.getAllByText((_, element) =>
        element?.textContent?.includes("Host draft: Night Debug") ?? false,
      ).length,
    ).toBeGreaterThan(0);
    expect(
      screen.getAllByText((_, element) =>
        element?.textContent?.includes("Current sequence: 17") ?? false,
      ).length,
    ).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "Launch extra client" }));

    expect(await screen.findByText("Launched debug-child-1")).toBeTruthy();
    expect(mockedLaunchAdditionalClientInstance).toHaveBeenCalledWith(null);
    expect(clipboardWriteText).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Launch extra client" })).toBeTruthy();
    expect(
      screen.getAllByText((_, element) =>
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

    expect(await screen.findByDisplayValue("pkr1_attached_payload")).toBeTruthy();
    expect(await screen.findByText(/legal Fold, Check, Bet/i)).toBeTruthy();

    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "Launch extra client with payload" }),
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
      screen.getAllByText((_, element) =>
        element?.textContent?.includes("Current sequence: 17") ?? false,
      ).length,
    ).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "Launch extra client with payload" })).toBeTruthy();
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
        blindPresetId: "standard",
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
        blindPresetId: "standard",
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
    fireEvent.click(screen.getByRole("button", { name: "Launch extra client" }));
    expect(await screen.findByText("Launched debug-child-1")).toBeTruthy();
    expect(localStorage.getItem(storageKey(parentBootstrap.storageNamespace, "ready-seats"))).toBe("[2]");
    expect(localStorage.getItem(storageKey(childBootstrap.storageNamespace, "ready-seats"))).toBe("[1]");
    parentRender.unmount();

    const childDebugRender = renderAppShell("/debug", childBootstrap);

    expect(await screen.findByText("Launch Child")).toBeTruthy();
    expect(await screen.findByText(/Spawn another debug client/i)).toBeTruthy();
    expect(
      screen.getAllByText((_, element) =>
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
      screen.getByRole("button", { name: "Continue to lobby" }).hasAttribute("disabled"),
    ).toBe(true);

    mockedGetTableView.mockRejectedValueOnce(
      new Error("Host connection lost. Reopen the lobby or rejoin."),
    );

    renderAppShell("/table");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
    expect(await screen.findByText("Host connection lost. Reopen the lobby or rejoin.")).toBeTruthy();

    fireEvent.click(screen.getByRole("link", { name: "Hand history" }));
    expect(
      await screen.findByRole("heading", { level: 2, name: "Hand History" }),
    ).toBeTruthy();
  });

  it("transitions an eliminated player into observer-only state after a live table action", async () => {
    mockedGetTableView.mockResolvedValueOnce(createTableViewSnapshot());
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
    expect(
      await screen.findByText(/maya won 240 chip\(s\)\./i),
    ).toBeTruthy();
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
