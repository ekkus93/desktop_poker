import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  createAppBootstrap,
  createParsedJoinPayload,
} from "../test/appIntegrationFixtures";

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

import { h } from "./appShellHarness";

describe("AppShell integration (host and lobby)", () => {
  beforeEach(() => {
    h.setupAppShellMocks();
  });

  it("moves from home to host setup and keeps the edited host draft in the lobby shell", async () => {
    h.renderAppShell("/");

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
      expect(h.mockedStartHostSession).toHaveBeenCalled();
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
    h.renderAppShell("/");

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
      expect(h.mockedStartHostSession).toHaveBeenCalled();
    });
    await waitFor(() => {
      expect(h.mockedAddNpcPlayers).toHaveBeenCalledTimes(1);
    });
    const npcRequest = h.mockedAddNpcPlayers.mock.calls[0][0];
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

  it("shows NPC add profile error inline without navigating away from setup", async () => {
    h.mockedAddNpcPlayers.mockRejectedValueOnce(
      new Error("could not load NPC profile 'bad-profile-id': No such file"),
    );

    h.renderAppShell("/");
    fireEvent.click(await screen.findByRole("link", { name: "Host Tournament" }));
    await screen.findByRole("heading", { level: 2, name: "Host Tournament Setup" });

    // Request 1 NPC so addNpcPlayers is called.
    fireEvent.change(await screen.findByLabelText(/npc players/i), {
      target: { value: "1" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Start hosting" }));

    // Error message must be visible.
    expect(
      await screen.findByText(/could not load NPC profile/i),
    ).toBeTruthy();

    // The host setup heading must still be present — we must NOT have navigated
    // to the lobby or any other route.
    expect(
      screen.queryByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeNull();
  });

  it("keeps a join flow in the lobby until a real table is running", async () => {
    h.renderAppShell("/join");

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
    h.renderAppShell("/join");

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
      expect(h.mockedClientClaimLobbySeat).toHaveBeenCalledWith({
        seatIndex: 2,
      });
    });
    expect(
      await screen.findByRole("button", { name: "I'm ready" }),
    ).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "I'm ready" }));

    await waitFor(() => {
      expect(h.mockedClientSetLobbyReadyState).toHaveBeenCalledWith({
        isReady: true,
      });
    });
    expect(screen.getByText("You: Ready")).toBeTruthy();
  });

  it("enables the host start control only when the live lobby is authoritatively ready", async () => {
    h.ctx.currentHostSession = h.buildHostSessionStatus({
      tournamentName: "Friday Night",
      displayName: "Host Alpha",
    });
    h.syncLiveSessions(
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

    h.renderAppShell("/lobby");

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
      expect(h.mockedHostStartTournament).toHaveBeenCalledTimes(1);
    });
    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
  });

  it("keeps the lobby in place when the authoritative host start rejects", async () => {
    h.ctx.currentHostSession = h.buildHostSessionStatus({
      tournamentName: "Friday Night",
      displayName: "Host Alpha",
    });
    h.syncLiveSessions(
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
    h.mockedHostStartTournament.mockRejectedValueOnce(
      new Error("host start rejected"),
    );

    h.renderAppShell("/lobby");

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
    h.mockedOnSessionUpdate.mockImplementation((cb) => {
      capturedSessionUpdateCallback = cb;
      return Promise.resolve(() => {});
    });

    h.ctx.currentHostSession = h.buildHostSessionStatus({
      tournamentName: "Friday Night",
      displayName: "Host Alpha",
    });

    h.renderAppShell("/lobby");

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
    h.ctx.currentHostSession = null;
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
    h.ctx.currentClientSession = h.buildClientSessionStatus();
    h.syncLiveSessions(
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

    h.renderAppShell("/lobby");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
  });

  it("creates, copies, and joins a tournament invite across host and join flows", async () => {
    const hostRender = h.renderAppShell("/host");

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
      expect(h.mockedStartHostSession).toHaveBeenCalledWith({
        hostAddress: "192.168.1.10",
        hostPort: 43818,
        tournamentName: "Invite Finals",
        maxPlayers: 6,
        startingStack: 1500,
        blindPresetId: "normal",
        turnTimerSeconds: 30,
        displayName: "Player test-instance",
      });
      expect(h.clipboardWriteText).toHaveBeenCalledWith("pkr1_host_invite");
    });

    hostRender.unmount();

    h.renderAppShell("/join");

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
    expect(h.mockedJoinHostSession).toHaveBeenCalledWith({
      joinPayload: "pkr1_host_invite",
      displayName: "Player test-instance",
    });
    expect(await screen.findByText("You: Waiting")).toBeTruthy();
  });

  it("redirects unknown routes back to the real home screen", async () => {
    h.renderAppShell("/does-not-exist");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();
  });

  it("keeps host setup blocked until the LAN address resolves", async () => {
    h.mockedResolveHostLanAddress.mockRejectedValueOnce(
      new Error("No reachable LAN IP"),
    );

    h.renderAppShell("/host");

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

    h.renderAppShell("/join", bootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Join Tournament" }),
    ).toBeTruthy();
    expect(await screen.findByLabelText("Invite preview")).toBeTruthy();
    expect(
      await screen.findByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();
  });

  // ---------------------------------------------------------------------------
  // Section 9 — NPC profile screen
  // ---------------------------------------------------------------------------

  it("npc profiles screen lists all profiles returned by the backend", async () => {
    // The NpcProfilesScreen calls listNpcProfiles() on mount and renders
    // each profile's name. Override the default empty mock with two profiles.
    h.mockedListNpcProfiles.mockResolvedValue([
      {
        id: "aggressive-alice",
        name: "Aggressive Alice",
        style: "loose-aggressive",
        skill: "advanced",
        description: "Always bets big.",
        opponentTendencies: null,
        tiltBehaviour: null,
      },
      {
        id: "conservative-carlos",
        name: "Conservative Carlos",
        style: "tight-passive",
        skill: "beginner",
        description: "Plays very few hands.",
        opponentTendencies: null,
        tiltBehaviour: null,
      },
    ]);

    h.renderAppShell("/npc-profiles");

    expect(
      await screen.findByRole("heading", { level: 2, name: "AI Profiles" }),
    ).toBeTruthy();
    expect(await screen.findByText("Aggressive Alice")).toBeTruthy();
    expect(screen.getByText("Conservative Carlos")).toBeTruthy();
  });
});
