import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  createAppBootstrap,
  createParsedJoinPayload,
  createTableViewSnapshot,
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

describe("AppShell integration (client and routing)", () => {
  beforeEach(() => {
    h.setupAppShellMocks();
  });

  it("keeps the lobby route behind an active live session", async () => {
    h.renderAppShell("/lobby");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();
    expect(
      screen.queryByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeNull();
    expect(screen.queryByText("You: Waiting")).toBeNull();
  });

  it("keeps the table route behind an active live session", async () => {
    h.renderAppShell("/table", createAppBootstrap(), {
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
    h.ctx.currentHostSession = {
      ...h.buildHostSessionStatus({ tournamentName: "Friday Night" }),
      phase: "waitingForPlayers",
    };

    h.renderAppShell("/table");

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
    h.ctx.currentHostSession = {
      ...h.buildHostSessionStatus({ tournamentName: "Friday Night" }),
      phase: "waitingForPlayers",
    };

    h.renderAppShell("/table", createAppBootstrap(), {
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
    h.ctx.currentHostSession = h.buildHostSessionStatus({
      tournamentName: "Friday Night",
    });

    h.renderAppShell("/lobby");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();

    fireEvent.click(await screen.findByRole("button", { name: "Close table" }));
    fireEvent.click(screen.getAllByRole("button", { name: "Close table" })[1]);

    await waitFor(() => {
      expect(h.mockedStopHostSession).toHaveBeenCalledTimes(1);
    });
    expect(
      await screen.findByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();
  });

  it("leaves the live client session before returning home", async () => {
    h.ctx.currentClientSession = h.buildClientSessionStatus();

    h.renderAppShell("/lobby");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();

    expect(await screen.findByText(/awaiting seat/i)).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Leave table" }));
    fireEvent.click(screen.getAllByRole("button", { name: "Leave table" })[1]);

    await waitFor(() => {
      expect(h.mockedLeaveClientSession).toHaveBeenCalledTimes(1);
    });
    expect(
      await screen.findByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();
  });

  it("reroutes removed ready-room paths back to the home screen", async () => {
    h.renderAppShell(
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
    h.renderAppShell("/table", createAppBootstrap(), {
      allowImplicitTableSession: true,
    });

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();

    await act(async () => {
      h.ctx.bootstrapSubscriptionHandler?.(
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

    h.renderAppShell("/join", bootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Join Tournament" }),
    ).toBeTruthy();
    expect(await screen.findByText("Invite signature mismatch")).toBeTruthy();
    expect(
      screen.queryByRole("button", { name: "Continue to lobby" }),
    ).toBeTruthy();

    await act(async () => {
      h.ctx.bootstrapSubscriptionHandler?.(
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
    h.mockedGetTableView.mockResolvedValue(
      createTableViewSnapshot({
        phaseLabel: "Complete",
        streetLabel: "Showdown",
        actionOwnerLabel: "Tournament complete",
        actionTray: null,
      }),
    );

    const firstRender = h.renderAppShell("/join", bootstrap);

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

    h.syncLiveSessions(
      h.ctx.currentClientSession?.participants ?? [],
      "running",
    );

    firstRender.unmount();

    h.renderAppShell("/table", bootstrap);

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

    h.mockedGetTableView.mockRejectedValue(new Error("offline"));
    h.renderAppShell("/history", bootstrap);

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
    const firstRender = h.renderAppShell(
      "/table",
      createAppBootstrap({ debugToolsEnabled: false }),
      { allowImplicitTableSession: true },
    );

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
    expect(await screen.findByLabelText("Ace of spades")).toBeTruthy();
    expect(
      screen.queryByRole("button", { name: "Observer preview" }),
    ).toBeNull();
    firstRender.unmount();

    h.renderAppShell("/table", createAppBootstrap({ debugToolsEnabled: true }));
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

    h.mockedGetTableView.mockReset();
    h.mockedGetTableView.mockResolvedValue(
      createTableViewSnapshot({
        actionOwnerLabel: "You",
      }),
    );

    const aliceRender = h.renderAppShell("/table", aliceBootstrap, {
      allowImplicitTableSession: true,
    });

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

    h.mockedGetTableView.mockResolvedValue(
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

    h.renderAppShell("/table", bobBootstrap);

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

  // ---------------------------------------------------------------------------
  // Section 10 — join flow error handling
  // ---------------------------------------------------------------------------

  it("join flow shows a human-readable error for an invalid payload instead of freezing", async () => {
    // validateJoinPayloadInput rejects with a descriptive error when the
    // payload string is structurally invalid. The screen must surface that
    // error and keep the Continue button disabled so the user is not confused.
    h.mockedValidateJoinPayloadInput.mockRejectedValueOnce(
      new Error("Invalid pkr1_ payload: base64 decoding failed"),
    );

    h.renderAppShell("/join");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Join Tournament" }),
    ).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Invite"), {
      target: { value: "pkr1_notbase64!!!" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Check invite" }));

    expect(
      await screen.findByText(/invalid pkr1_ payload: base64 decoding failed/i),
    ).toBeTruthy();
    expect(
      screen
        .getByRole("button", { name: "Continue to lobby" })
        .hasAttribute("disabled"),
    ).toBe(true);
    expect(
      screen.queryByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeNull();
  });

  it("join flow shows a connection error when the host is unreachable", async () => {
    // validateJoinPayloadInput succeeds (structurally valid payload), but
    // joinHostSession fails because the host is not reachable.  The Join
    // screen must surface the error instead of navigating to the lobby.
    h.mockedValidateJoinPayloadInput.mockResolvedValueOnce(
      createParsedJoinPayload(),
    );
    h.mockedJoinHostSession.mockRejectedValueOnce(
      new Error("Connection refused"),
    );

    h.renderAppShell("/join");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Join Tournament" }),
    ).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Invite"), {
      target: { value: "pkr1_validlooking" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Check invite" }));

    // Wait for validation to succeed (button unlocks).
    const continueBtn = await screen.findByRole("button", {
      name: "Continue to lobby",
    });
    expect(continueBtn.hasAttribute("disabled")).toBe(false);

    fireEvent.click(continueBtn);

    expect(await screen.findByText(/connection refused/i)).toBeTruthy();
    expect(
      screen.queryByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeNull();
  });
});
