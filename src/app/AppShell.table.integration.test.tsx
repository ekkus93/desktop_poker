import { act, fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  createAppBootstrap,
  createTableViewSnapshot,
} from "../test/appIntegrationFixtures";
import { storageKey } from "./shell";

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

describe("AppShell integration (table and debug)", () => {
  beforeEach(() => {
    h.setupAppShellMocks();
  });

  it("keeps the live table snapshot stable when an action fails and recovers on the next successful retry", async () => {
    h.mockedGetTableView.mockResolvedValue(createTableViewSnapshot());
    h.mockedSubmitTableAction
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

    h.renderAppShell("/table", createAppBootstrap(), {
      allowImplicitTableSession: true,
    });

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
    h.mockedGetTableView.mockResolvedValue(
      createTableViewSnapshot({
        actionOwnerLabel: "Maya",
        actionTray: null,
      }),
    );

    const tableRender = h.renderAppShell("/table", createAppBootstrap(), {
      allowImplicitTableSession: true,
    });

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

    h.mockedGetTableView.mockResolvedValue(createTableViewSnapshot());
    h.mockedSubmitTableAction.mockRejectedValueOnce(
      new Error("Action window expired."),
    );

    const aliceRender = h.renderAppShell("/table", aliceBootstrap, {
      allowImplicitTableSession: true,
    });

    fireEvent.click(await screen.findByRole("button", { name: "Fold" }));
    expect(await screen.findByText("Action window expired.")).toBeTruthy();
    aliceRender.unmount();

    h.renderAppShell("/table", bobBootstrap);

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

    h.mockedGetTableView.mockResolvedValue(createTableViewSnapshot());

    h.renderAppShell("/table", bootstrap, { allowImplicitTableSession: true });

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

    h.mockedGetTableView.mockRejectedValue(
      new Error("Host connection lost. Reopen the lobby or rejoin."),
    );

    h.renderAppShell("/table", bootstrap, { allowImplicitTableSession: true });

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

    h.mockedGetTableView.mockRejectedValue(new Error("offline"));

    h.renderAppShell("/history", secondBootstrap);

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

    h.renderAppShell("/debug", bootstrap);

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
    expect(h.mockedLaunchAdditionalClientInstance).toHaveBeenCalledWith(null);
    expect(h.clipboardWriteText).not.toHaveBeenCalled();
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

    h.renderAppShell("/debug", bootstrap);

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
    expect(h.clipboardWriteText).toHaveBeenCalledWith("pkr1_attached_payload");
    expect(h.mockedLaunchAdditionalClientInstance).toHaveBeenCalledWith(
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

    const parentRender = h.renderAppShell("/debug", parentBootstrap);

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

    const childDebugRender = h.renderAppShell("/debug", childBootstrap);

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

    h.mockedGetTableView.mockRejectedValueOnce(new Error("offline"));

    h.renderAppShell("/history", childBootstrap);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Hand History" }),
    ).toBeTruthy();
    expect(screen.getByText(/launch child won 440 chip\(s\)\./i)).toBeTruthy();
    expect(screen.queryByText(/parent room/i)).toBeNull();
  });

  it("keeps join and table failures inside explicit shell states instead of stranding navigation", async () => {
    h.mockedValidateJoinPayloadInput.mockRejectedValueOnce(
      new Error("Join payload rejected"),
    );

    h.renderAppShell("/join");

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

    h.mockedGetTableView.mockRejectedValue(
      new Error("Host connection lost. Reopen the lobby or rejoin."),
    );

    h.renderAppShell("/table", createAppBootstrap(), {
      allowImplicitTableSession: true,
    });

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
    h.mockedGetTableView.mockResolvedValue(createTableViewSnapshot());
    h.mockedSubmitTableAction.mockResolvedValueOnce(
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

    h.renderAppShell("/table", createAppBootstrap(), {
      allowImplicitTableSession: true,
    });

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
    h.renderAppShell("/");

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
  });

  it("shows timeout error inline and keeps action tray available after host ack timeout", async () => {
    // Simulates the backend returning a timeout error (P0.5): the UI must
    // surface the message and NOT navigate away or hide the action tray.
    h.mockedGetTableView.mockResolvedValue(createTableViewSnapshot());
    h.mockedSubmitTableAction.mockRejectedValueOnce(
      new Error(
        "table action timed out: host did not acknowledge within 1 second",
      ),
    );

    h.renderAppShell("/table", createAppBootstrap(), {
      allowImplicitTableSession: true,
    });

    expect(await screen.findByRole("button", { name: "Fold" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Fold" }));

    // Timeout error must be visible as an inline banner.
    expect(
      await screen.findByText(/table action timed out/i),
    ).toBeTruthy();

    // Action tray must still be present so the player can retry.
    expect(screen.getByRole("button", { name: "Check" })).toBeTruthy();

    // We must still be on the table route — not rerouted to home.
    expect(
      screen.queryByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeNull();
  });
});
