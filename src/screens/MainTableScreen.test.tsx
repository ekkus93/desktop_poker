import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getTableView,
  onTableUpdate,
  submitTableAction,
  type DesktopTableActionKind,
  type TableViewSnapshot,
  type TableViewerMode,
} from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { createTableViewSnapshot } from "../test/appIntegrationFixtures";
import { MainTableScreen } from "./MainTableScreen";

vi.mock("../api/desktop", async () => {
  const actual =
    await vi.importActual<typeof import("../api/desktop")>("../api/desktop");

  return {
    ...actual,
    getTableView: vi.fn(),
    submitTableAction: vi.fn(),
    onTableUpdate: vi.fn().mockResolvedValue(() => {}),
  };
});

const mockedGetTableView = vi.mocked(getTableView);
const mockedSubmitTableAction = vi.mocked(submitTableAction);
const mockedOnTableUpdate = vi.mocked(onTableUpdate);

const { mockNavigate } = vi.hoisted(() => ({ mockNavigate: vi.fn() }));

vi.mock("react-router", async () => {
  const actual =
    await vi.importActual<typeof import("react-router")>("react-router");

  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

const createTableView = createTableViewSnapshot;

describe("MainTableScreen", () => {
  beforeEach(() => {
    mockedGetTableView.mockReset();
    mockedSubmitTableAction.mockReset();
    mockedOnTableUpdate.mockReset();
    mockedOnTableUpdate.mockResolvedValue(() => {});
    mockNavigate.mockReset();
    vi.useRealTimers();
  });

  it("enables the action tray only for the acting local player without exposing observer preview controls", async () => {
    const localView = createTableView();
    mockedGetTableView.mockResolvedValue(localView);

    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    const firstRender = renderWithProviders(
      <MainTableScreen bootstrap={bootstrap} />,
      {
        bootstrap,
      },
    );

    expect(
      (await screen.findByRole("button", {
        name: "Fold",
      })) as HTMLButtonElement,
    ).toHaveProperty("disabled", false);
    expect(
      screen.getByRole("button", { name: "Check" }) as HTMLButtonElement,
    ).toHaveProperty("disabled", false);
    expect(screen.getByText("A♠")).toBeTruthy();

    expect(
      screen.queryByRole("button", { name: "Observer preview" }),
    ).toBeNull();
    firstRender.unmount();
  });

  it("shows an honest pre-start table state instead of fake board cards", async () => {
    mockedGetTableView.mockResolvedValue(
      createTableView({
        phaseLabel: "Ready check",
        streetLabel: "Waiting",
        currentHandNumber: null,
        boardCards: [],
        actionTray: null,
        potTotal: 0,
        actionOwnerLabel: "Waiting for the tournament to start",
      }),
    );

    const bootstrap = createBootstrap();
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    expect(await screen.findByText("Tournament has not started")).toBeTruthy();
    expect(
      screen.getByText(
        /the tournament is live but the first hand has not started yet/i,
      ),
    ).toBeTruthy();
    expect(screen.queryByLabelText("Community cards")).toBeNull();
    expect(screen.queryByRole("button", { name: "Fold" })).toBeNull();
  });

  it("reflects public events, history, and standings after settlement", async () => {
    const initialView = createTableView();
    const settledView = createTableView({
      streetLabel: "Turn",
      boardCards: [
        ...createTableView().boardCards,
        {
          label: "Jack of spades",
          compactLabel: "J♠",
          suitSymbol: "♠",
          tone: "dark",
        },
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
        {
          sequence: 18,
          kind: "public-event",
          message: "The turn was published to every seat and observer.",
        },
        {
          sequence: 19,
          kind: "settlement",
          message: "Hand 9 settled: Maya collected the pot.",
        },
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
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    fireEvent.click(await screen.findByRole("button", { name: "Check" }));
    fireEvent.click(screen.getByRole("button", { name: "Table details" }));

    expect(
      await screen.findByText(
        /the turn was published to every seat and observer/i,
      ),
    ).toBeTruthy();
    expect(screen.getByText("Pot 240")).toBeTruthy();
    expect(screen.getByText(/maya won 240 chip\(s\)/i)).toBeTruthy();
    expect(screen.getByText(/#1 Maya/)).toBeTruthy();
    expect(
      screen.getByText(/waiting for the next action from the host/i),
    ).toBeTruthy();
  });

  it("refreshes the live table view so passive seats observe remote actions", async () => {
    const initialView = createTableView({
      actionOwnerLabel: "Maya",
      potTotal: 120,
      actionTray: null,
    });
    const refreshedView = createTableView({
      streetLabel: "Turn",
      potTotal: 240,
      actionOwnerLabel: "Host",
      actionTray: null,
      eventFeed: [
        {
          sequence: 19,
          kind: "public-event",
          message:
            "Host called and the turn was published to every seat and observer.",
        },
      ],
    });

    // Capture the table-update callback so we can fire it manually
    let capturedTableUpdateCallback: (() => void) | undefined;
    mockedOnTableUpdate.mockImplementation((cb) => {
      capturedTableUpdateCallback = cb;
      return Promise.resolve(() => {});
    });

    mockedGetTableView
      .mockResolvedValueOnce(initialView)
      .mockResolvedValue(refreshedView);

    const bootstrap = createBootstrap();
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    expect(
      await screen.findByRole("heading", { level: 2, name: "Main Table" }),
    ).toBeTruthy();
    expect(await screen.findByText("Pot 120")).toBeTruthy();

    // Fire the table-update event to trigger a refresh (simulates the Rust backend emitting)
    await waitFor(() => expect(capturedTableUpdateCallback).toBeDefined());
    capturedTableUpdateCallback?.();

    await waitFor(
      () => {
        expect(screen.getByText("Pot 240")).toBeTruthy();
      },
      { timeout: 2000 },
    );
    fireEvent.click(screen.getByRole("button", { name: "Table details" }));
    expect(
      screen.getByText(
        /host called and the turn was published to every seat and observer/i,
      ),
    ).toBeTruthy();
  });

  it("submits raises directly and still confirms all-in actions", async () => {
    const initialView = createTableView();
    mockedGetTableView.mockResolvedValue(initialView);
    mockedSubmitTableAction.mockResolvedValue(
      createTableView({ actionTray: null, actionOwnerLabel: "Maya" }),
    );

    const bootstrap = createBootstrap();
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    fireEvent.click(
      await screen.findByRole("button", { name: /bet \/ raise/i }),
    );

    await waitFor(() => {
      expect(mockedSubmitTableAction).toHaveBeenCalledWith(
        "local",
        "betOrRaise",
        60,
      );
    });
    expect(screen.queryByText(/sending your action to the host/i)).toBeNull();

    mockedSubmitTableAction.mockClear();
    mockedGetTableView.mockResolvedValue(initialView);
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

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

  it("explains locked raise actions and shows a submission banner while waiting", async () => {
    const initialView = createTableView({
      actionTray: {
        ...createTableView().actionTray!,
        maxRaiseTo: null,
      },
    });
    let resolveAction!: (value: TableViewSnapshot) => void;
    mockedGetTableView.mockResolvedValue(initialView);
    mockedSubmitTableAction.mockImplementation(
      () =>
        new Promise<TableViewSnapshot>((resolve) => {
          resolveAction = resolve;
        }),
    );

    const bootstrap = createBootstrap();
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    expect(
      await screen.findByText(
        /raise unavailable until a legal raise size is offered for this spot/i,
      ),
    ).toBeTruthy();
    expect(
      screen
        .getByRole("button", { name: /bet \/ raise/i })
        .hasAttribute("disabled"),
    ).toBe(true);

    fireEvent.click(screen.getByRole("button", { name: "Fold" }));
    expect(
      await screen.findByText(/sending your action to the host/i),
    ).toBeTruthy();

    resolveAction(
      createTableView({ actionTray: null, actionOwnerLabel: "Maya" }),
    );

    await waitFor(() => {
      expect(screen.queryByText(/sending your action to the host/i)).toBeNull();
    });
  });

  it("shows statusLabel not chip count for observer entries in standings (B1)", async () => {
    mockedGetTableView.mockResolvedValue(createTableView());
    const bootstrap = createBootstrap();
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    fireEvent.click(
      await screen.findByRole("button", { name: "Table details" }),
    );

    // Riley is isObserver: true in the fixture — standings entry should show statusLabel.
    // "Eliminated observer" text appears in both the seat card and the standings list.
    expect(screen.getAllByText("Eliminated observer").length).toBeGreaterThan(
      0,
    );

    // Find Riley's standings row by rank heading and verify its field-hint contains
    // the statusLabel, not a chip count.
    const rileyRankHeading = screen.getByText(/#4 Riley/);
    const rileyArticle = rileyRankHeading.closest("article");
    expect(rileyArticle?.textContent).toContain("Eliminated observer");
    expect(rileyArticle?.textContent).not.toMatch(/\d+ chips/);
  });

  it("caps the event feed at 50 entries and shows a note when capped (B3)", async () => {
    const manyEvents = Array.from({ length: 55 }, (_, i) => ({
      sequence: i,
      kind: "public-event",
      message: `Event number ${i}`,
    }));
    mockedGetTableView.mockResolvedValue(
      createTableView({ eventFeed: manyEvents }),
    );
    const bootstrap = createBootstrap();
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    fireEvent.click(
      await screen.findByRole("button", { name: "Table details" }),
    );

    // Events 0-4 should be dropped (slice(-50) keeps 5-54)
    expect(screen.queryByText("Event number 0")).toBeNull();
    expect(screen.getByText("Event number 5")).toBeTruthy();
    expect(screen.getByText("Event number 54")).toBeTruthy();
    expect(screen.getByText(/showing last 50 events/i)).toBeTruthy();
  });

  it("hides the raise slider and shows no NaN labels when raise bounds are null (B4)", async () => {
    mockedGetTableView.mockResolvedValue(
      createTableView({
        actionTray: {
          ...createTableView().actionTray!,
          minRaiseTo: null,
          maxRaiseTo: null,
        },
      }),
    );
    const bootstrap = createBootstrap();
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    await screen.findByRole("button", { name: "Fold" });

    expect(screen.queryByRole("slider", { name: /raise/i })).toBeNull();
    expect(screen.queryByText("NaN")).toBeNull();
    expect(
      screen.getByText(
        /raise unavailable until a legal raise size is offered for this spot/i,
      ),
    ).toBeTruthy();
  });

  it("renders empty community card slots without visible Board N text (I2)", async () => {
    mockedGetTableView.mockResolvedValue(
      createTableView({ boardCards: [], currentHandNumber: 3 }),
    );
    const bootstrap = createBootstrap();
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    // Community cards section should appear but contain no "Board N" visible text
    const communityCards = await screen.findByLabelText("Community cards");
    expect(communityCards).toBeTruthy();
    expect(communityCards.textContent).not.toMatch(/Board \d/);
  });

  it("shows hand number in observer banner when the local player is an observer (I4)", async () => {
    mockedGetTableView.mockResolvedValue(
      createTableView({
        observerBanner: "You are watching as an observer.",
        currentHandNumber: 7,
      }),
    );
    const bootstrap = createBootstrap();
    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    expect(await screen.findByText(/watching hand #7/i)).toBeTruthy();
  });

  it("keeps focus order sane across the main action tray", async () => {
    const bootstrap = createBootstrap();
    const user = userEvent.setup();
    mockedGetTableView.mockResolvedValue(createTableView());

    renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    const foldButton = await screen.findByRole("button", { name: "Fold" });
    const checkButton = screen.getByRole("button", { name: "Check" });
    const raiseButton = screen.getByRole("button", { name: /bet \/ raise/i });
    const allInButton = screen.getByRole("button", { name: "All-in" });

    while (document.activeElement !== foldButton) {
      await user.tab();
    }

    expect(document.activeElement).toBe(foldButton);
    await user.tab();
    expect(document.activeElement).toBe(checkButton);
    await user.tab();
    expect(document.activeElement).toBe(raiseButton);
    await user.tab();
    expect(document.activeElement).toBe(allInButton);
  });

  it("backs off to the slow poll interval and shows the connection-slow banner after 3 consecutive poll errors (B6)", async () => {
    vi.useFakeTimers();
    const liveView = createTableView();
    mockedGetTableView
      .mockResolvedValueOnce(liveView) // initial load succeeds
      .mockRejectedValue(new Error("network down")); // every poll fails

    const bootstrap = createBootstrap();
    await act(async () => {
      renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
        bootstrap,
      });
    });

    // The first two poll failures stay on the 5s interval with no slow banner.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });
    expect(screen.queryByText(/connection slow/i)).toBeNull();

    // The third consecutive failure crosses the backoff threshold.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });
    expect(screen.getByText(/connection slow/i)).toBeTruthy();
    expect(mockNavigate).not.toHaveBeenCalled();

    vi.useRealTimers();
  });

  it("stops polling and navigates to the error screen after 10 consecutive poll errors (B6)", async () => {
    vi.useFakeTimers();
    const liveView = createTableView();
    mockedGetTableView
      .mockResolvedValueOnce(liveView)
      .mockRejectedValue(new Error("network down"));

    const bootstrap = createBootstrap();
    await act(async () => {
      renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
        bootstrap,
      });
    });

    // Drive past the 10-error limit (three failures at 5s, the rest at the 10s
    // slow interval). The tenth failure triggers the error-screen navigation.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(120000);
    });
    expect(mockNavigate).toHaveBeenCalledWith("/errors", { replace: true });

    vi.useRealTimers();
  });

  it("resets the error counter and clears the slow banner after a successful poll (B6)", async () => {
    vi.useFakeTimers();
    const liveView = createTableView();
    const failure = new Error("network down");
    mockedGetTableView
      .mockResolvedValueOnce(liveView) // initial load
      .mockRejectedValueOnce(failure) // poll #1
      .mockRejectedValueOnce(failure) // poll #2
      .mockRejectedValueOnce(failure) // poll #3 -> slow banner
      .mockResolvedValue(liveView); // poll #4 recovers

    const bootstrap = createBootstrap();
    await act(async () => {
      renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
        bootstrap,
      });
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(15000); // three failures at 5s each
    });
    expect(screen.getByText(/connection slow/i)).toBeTruthy();

    // After backoff the next poll runs at the 10s slow interval and succeeds,
    // resetting the counter and clearing the banner without navigating away.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10000);
    });
    expect(screen.queryByText(/connection slow/i)).toBeNull();
    expect(mockNavigate).not.toHaveBeenCalled();

    vi.useRealTimers();
  });
});
