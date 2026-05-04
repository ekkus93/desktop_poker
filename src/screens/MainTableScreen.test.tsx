import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getTableView,
  submitTableAction,
  type DesktopTableActionKind,
  type TableViewerMode,
} from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { createTableViewSnapshot } from "../test/appIntegrationFixtures";
import { MainTableScreen } from "./MainTableScreen";

vi.mock("../api/desktop", async () => {
  const actual = await vi.importActual<typeof import("../api/desktop")>(
    "../api/desktop",
  );

  return {
    ...actual,
    getTableView: vi.fn(),
    submitTableAction: vi.fn(),
  };
});

const mockedGetTableView = vi.mocked(getTableView);
const mockedSubmitTableAction = vi.mocked(submitTableAction);

const createTableView = createTableViewSnapshot;

describe("MainTableScreen", () => {
  beforeEach(() => {
    mockedGetTableView.mockReset();
    mockedSubmitTableAction.mockReset();
  });

  it("enables the action tray only for the acting local player and keeps observer preview in debug only", async () => {
    const localView = createTableView();
    const observerView = createTableView({
      viewerMode: "observer",
      observerBanner:
        "Observer mode uses the public projector only: no private hole cards and no actions.",
      actionTray: null,
      actionOwnerLabel: "Maya",
      seats: createTableView().seats.map((seat) =>
        seat.isLocal ? { ...seat, cardsHidden: true } : seat,
      ),
    });

    mockedGetTableView.mockImplementation((viewerMode: TableViewerMode) =>
      Promise.resolve(viewerMode === "observer" ? observerView : localView),
    );

    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    const firstRender = renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    expect(
      (await screen.findByRole("button", { name: "Fold" })) as HTMLButtonElement,
    ).toHaveProperty("disabled", false);
    expect(
      screen.getByRole("button", { name: "Check" }) as HTMLButtonElement,
    ).toHaveProperty("disabled", false);
    expect(screen.getByText("A♠")).toBeTruthy();

    expect(screen.queryByRole("button", { name: "Observer preview" })).toBeNull();
    firstRender.unmount();

    const debugBootstrap = createBootstrap({ debugToolsEnabled: true });
    renderWithProviders(<MainTableScreen bootstrap={debugBootstrap} />, {
      bootstrap: debugBootstrap,
    });

    fireEvent.click(await screen.findByRole("button", { name: "Observer preview" }));

    expect(
      await screen.findByText(/observer mode uses the public projector only/i),
    ).toBeTruthy();
    await waitFor(() => {
      expect(screen.queryByRole("button", { name: "Fold" })).toBeNull();
    });
    expect(
      screen.getByText(/observer preview keeps the table public-only/i),
    ).toBeTruthy();
  });

  it("reflects public events, history, and standings after settlement", async () => {
    const initialView = createTableView();
    const settledView = createTableView({
      streetLabel: "Turn",
      boardCards: [
        ...createTableView().boardCards,
        { label: "Jack of spades", compactLabel: "J♠", suitSymbol: "♠", tone: "dark" },
      ],
      potTotal: 240,
      actionOwnerLabel: "Waiting for settlement",
      standings: [
        {
          rank: 1,
          displayName: "Maya",
          chipCount: 1680,
          statusLabel: "Active",
          note: null,
          isLocal: false,
          isObserver: false,
        },
        {
          rank: 2,
          displayName: "You",
          chipCount: 1480,
          statusLabel: "Active",
          note: null,
          isLocal: true,
          isObserver: false,
        },
        ...createTableView().standings.slice(2),
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
        ...createTableView().handHistory,
      ],
      eventFeed: [
        { sequence: 18, kind: "public-event", message: "The turn was published to every seat and observer." },
        { sequence: 19, kind: "settlement", message: "Hand 9 settled: Maya collected the pot." },
      ],
      actionTray: null,
    });

    mockedGetTableView.mockResolvedValue(initialView);
    mockedSubmitTableAction.mockImplementation(
      async (
        viewerMode: TableViewerMode,
        actionKind: DesktopTableActionKind,
      ) => {
        expect(viewerMode).toBe("local");
        expect(actionKind).toBe("checkOrCall");
        return settledView;
      },
    );

    const bootstrap = createBootstrap();
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, { bootstrap });

    fireEvent.click(await screen.findByRole("button", { name: "Check" }));
    fireEvent.click(screen.getByRole("button", { name: "Show details" }));

    expect(
      await screen.findByText(/the turn was published to every seat and observer/i),
    ).toBeTruthy();
    expect(screen.getByText("J♠")).toBeTruthy();
    expect(screen.getByText(/maya won 240 chip\(s\)/i)).toBeTruthy();
    expect(screen.getByText(/#1 Maya/)).toBeTruthy();
    expect(
      screen.getByText(/waiting for the next local action window/i),
    ).toBeTruthy();
  });

  it("requires confirmation for raise and all-in actions", async () => {
    const initialView = createTableView();
    mockedGetTableView.mockResolvedValue(initialView);
    mockedSubmitTableAction.mockResolvedValue(
      createTableView({ actionTray: null, actionOwnerLabel: "Maya" }),
    );

    const bootstrap = createBootstrap();
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, { bootstrap });

    fireEvent.click(await screen.findByRole("button", { name: /bet \/ raise/i }));
    expect(await screen.findByText(/confirm raise/i)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(mockedSubmitTableAction).toHaveBeenCalledWith(
        "local",
        "betOrRaise",
        60,
      );
    });

    mockedSubmitTableAction.mockClear();
    mockedGetTableView.mockResolvedValue(initialView);
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, { bootstrap });

    fireEvent.click(await screen.findByRole("button", { name: "All-in" }));
    expect(await screen.findByText(/confirm all-in/i)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(mockedSubmitTableAction).toHaveBeenCalledWith(
        "local",
        "allIn",
        undefined,
      );
    });
  });
});
