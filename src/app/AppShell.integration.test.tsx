import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchBootstrapState,
  getTableView,
  resolveHostLanAddress,
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

vi.mock("../api/desktop", async () => {
  const actual = await vi.importActual<typeof import("../api/desktop")>(
    "../api/desktop",
  );

  return {
    ...actual,
    fetchBootstrapState: vi.fn(),
    subscribeBootstrap: vi.fn(),
    resolveHostLanAddress: vi.fn(),
    validateJoinPayloadInput: vi.fn(),
    getTableView: vi.fn(),
    submitTableAction: vi.fn(),
  };
});

const mockedFetchBootstrapState = vi.mocked(fetchBootstrapState);
const mockedSubscribeBootstrap = vi.mocked(subscribeBootstrap);
const mockedResolveHostLanAddress = vi.mocked(resolveHostLanAddress);
const mockedValidateJoinPayloadInput = vi.mocked(validateJoinPayloadInput);
const mockedGetTableView = vi.mocked(getTableView);
const mockedSubmitTableAction = vi.mocked(submitTableAction);

function renderAppShell(initialEntry: string, bootstrap = createAppBootstrap()) {
  mockedFetchBootstrapState.mockResolvedValue(bootstrap);
  mockedSubscribeBootstrap.mockResolvedValue(() => {});

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
    mockedFetchBootstrapState.mockReset();
    mockedSubscribeBootstrap.mockReset();
    mockedResolveHostLanAddress.mockReset();
    mockedValidateJoinPayloadInput.mockReset();
    mockedGetTableView.mockReset();
    mockedSubmitTableAction.mockReset();
    mockedResolveHostLanAddress.mockResolvedValue("192.168.1.10");
    mockedValidateJoinPayloadInput.mockResolvedValue(createParsedJoinPayload());
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

    expect(screen.getByLabelText(/copy invite details/i)).toBeTruthy();
    expect(screen.getByText("Friday Finals")).toBeTruthy();
    expect(screen.getByText(/192\.168\.1\.10:43818/i)).toBeTruthy();

    fireEvent.click(
      screen.getByRole("link", { name: "Continue to lobby" }),
    );

    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Lobby",
      }),
    ).toBeTruthy();
    expect(screen.getAllByText("Friday Finals").length).toBeGreaterThan(0);
  });

  it("runs a join-to-table flow through lobby, table, and history", async () => {
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
      screen.getByRole("link", { name: "Continue to lobby" }),
    );

    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Lobby",
      }),
    ).toBeTruthy();
    expect(screen.getAllByText(/waiting for player/i).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "Mark ready" }));
    fireEvent.click(screen.getByRole("button", { name: "Host marks ready" }));

    expect(
      screen.getByRole("link", { name: "Start tournament" }),
    ).toBeTruthy();

    fireEvent.click(screen.getByRole("link", { name: "Start tournament" }));

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
    expect(await screen.findByRole("button", { name: "Fold" })).toBeTruthy();

    fireEvent.click(screen.getByRole("link", { name: "Hand history" }));

    expect(
      await screen.findByRole("heading", { level: 2, name: "Hand History" }),
    ).toBeTruthy();
    expect(screen.getByText(/host won 210 chip\(s\)\./i)).toBeTruthy();
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
    expect(screen.getByRole("link", { name: "Continue to lobby" })).toBeTruthy();
  });

  it("keeps the lobby readiness badge aligned with the local seat state", async () => {
    renderAppShell("/lobby");

    expect(
      await screen.findByRole("heading", { level: 2, name: "Lobby" }),
    ).toBeTruthy();
    expect(screen.getByText("You: Waiting")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Mark ready" }));

    expect(await screen.findByText("You: Ready")).toBeTruthy();
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
      screen.getByRole("link", { name: "Continue to lobby" }),
    );
    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Lobby",
      }),
    ).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Mark ready" }));
    fireEvent.click(screen.getByRole("button", { name: "Host marks ready" }));
    fireEvent.click(screen.getByRole("link", { name: "Start tournament" }));

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
    fireEvent.click(screen.getByRole("link", { name: "Tournament complete" }));

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
    expect(screen.getByText(/host won 210 chip\(s\)\./i)).toBeTruthy();
    expect(
      Object.keys(localStorage).some((key) => key.toLowerCase().includes("reconnect")),
    ).toBe(false);
  });

  it("keeps observer preview behind debug mode while preserving table routing", async () => {
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

    fireEvent.click(screen.getByRole("button", { name: "Observer preview" }));

    expect(
      await screen.findByText(/observer mode uses the public projector only/i),
    ).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Fold" })).toBeNull();

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
    expect(await screen.findByText("120 chips")).toBeTruthy();
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
    expect(await screen.findByText("120 chips")).toBeTruthy();
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
      screen.queryByRole("link", { name: "Continue to lobby" }),
    ).toBeNull();

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
