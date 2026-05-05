import { expect, test, type Locator, type Page } from "@playwright/test";

type Rect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

async function getRect(locator: Locator): Promise<Rect> {
  const box = await locator.boundingBox();

  expect(box).not.toBeNull();

  return box as Rect;
}

async function openProbe(page: Page, surface: string) {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto(`/?layout-probe=${surface}`);
}

async function installRoutedShellMocks(page: Page) {
  await page.addInitScript(() => {
    const bootstrap = {
      appName: "Desktop Poker",
      protocolVersion: 1,
      defaultHostPort: 43818,
      frontendStack: "React + TypeScript",
      serializationStrategy: "serde + canonical JSON bytes",
      framingStrategy: "length-prefixed JSON envelopes",
      joinPayloadEncoding: "pkr1_ compact join payload",
      runtimeTransport: "raw TCP over LAN",
      cryptoStack: ["ed25519-dalek", "x25519-dalek", "chacha20poly1305"],
      instanceId: "playwright-routed-host",
      instanceLabel: "playwright-routed-host",
      storageNamespace: "desktop-poker:playwright-routed-host",
      sessionIdentity: "desktop-session:playwright-routed-host",
      reconnectNamespace: "desktop-reconnect:playwright-routed-host",
      profileDirectory: "/tmp/desktop-poker/playwright-routed-host",
      launchJoinPayload: null,
      parsedLaunchJoinPayload: null,
      launchJoinPayloadError: null,
      debugToolsEnabled: false,
      backendModules: [],
      screens: [
        { id: "home", title: "Home", route: "/", surface: "primary" },
        { id: "host", title: "Host", route: "/host", surface: "primary" },
        { id: "join", title: "Join", route: "/join", surface: "primary" },
        { id: "history", title: "History", route: "/history", surface: "support" },
        { id: "rules", title: "Help", route: "/rules", surface: "support" },
      ],
    };

    const joinPayload = {
      payloadVersion: 1,
      hostAddress: "192.168.1.10",
      hostPort: 43818,
      tableId: "playwright-routed-host-table",
      sessionEpoch: 1,
      hostSigningPublicKey: "probe-public-key",
      joinToken: "probe-join-token",
      generatedAtMs: 1,
      tableName: "Friday Night",
    };

    const tableView = {
      viewerMode: "local",
      tournamentName: "Desktop Sit 'n Go playwright-routed-host",
      tableName: "Main Table",
      tableId: "playwright-routed-host-table",
      phaseLabel: "Running",
      streetLabel: "Flop",
      blindLevelLabel: "Level 1 · 10 / 20",
      currentHandNumber: 9,
      boardCards: [],
      potTotal: 120,
      actionOwnerLabel: "You",
      eliminationSummary: "Everyone still has chips.",
      observerBanner: null,
      seats: [],
      standings: [],
      handHistory: [
        {
          handNumber: 8,
          summary: "Host won 210 chip(s).",
          potTotal: 210,
          winningPlayers: ["Host"],
          eliminatedPlayers: [],
          boardCards: [],
        },
      ],
      eventFeed: [
        {
          sequence: 18,
          kind: "public-event",
          message: "The flop was published to every seat and observer.",
        },
      ],
      actionTray: null,
    };

    window.__DESKTOP_POKER_BROWSER_MOCKS__ = {
      fetchBootstrapState: async () => bootstrap,
      subscribeBootstrap: async () => () => {},
      resolveHostLanAddress: async () => "192.168.1.10",
      validateJoinPayloadInput: async () => joinPayload,
      getTableView: async () => tableView,
      submitTableAction: async () => tableView,
      getDebugState: async () => ({
        protocolLog: [],
        snapshotJson: "{}",
        currentSequence: 1,
        currentHandNumber: tableView.currentHandNumber,
        actionWindowSummary: null,
        launchHint: "playwright-routed-host",
      }),
      launchAdditionalClientInstance: async () => "playwright-routed-host-client",
    };

    const prefix = `${bootstrap.storageNamespace}:`;
    localStorage.setItem(`${prefix}recent-join-payloads`, JSON.stringify([joinPayload.joinToken]));
    localStorage.setItem(`${prefix}hand-history`, JSON.stringify(tableView.handHistory));
  });
}

function expectRightColumn(main: Rect, side: Rect) {
  expect(side.x).toBeGreaterThan(main.x + main.width * 0.55);
  expect(Math.abs(side.y - main.y)).toBeLessThanOrEqual(8);
}

test.describe("layout probe browser geometry", () => {
  test("keeps the routed host shell inside the viewport without scroll", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await installRoutedShellMocks(page);
    await page.goto("/#/host");
    await expect(
      page.getByRole("heading", { level: 2, name: "Host Tournament Setup" }),
    ).toBeVisible();

    const metrics = await page.evaluate(() => ({
      viewportHeight: window.innerHeight,
      scrollHeight: document.documentElement.scrollHeight,
      workstationTop:
        document.querySelector(".workstation-main-card")?.getBoundingClientRect().top ?? 0,
      workstationBottom:
        document.querySelector(".workstation-main-card")?.getBoundingClientRect().bottom ?? 0,
    }));

    expect(metrics.workstationTop).toBeGreaterThan(0);
    expect(metrics.workstationBottom).toBeLessThanOrEqual(metrics.viewportHeight);
    expect(metrics.scrollHeight).toBeLessThanOrEqual(metrics.viewportHeight);
  });

  test("keeps the host setup card inside the desktop viewport", async ({ page }) => {
    await openProbe(page, "host");

    const metrics = await page.evaluate(() => ({
      viewportHeight: window.innerHeight,
      cardBottom:
        document.querySelector(".workstation-main-card")?.getBoundingClientRect().bottom ?? 0,
      cardTop:
        document.querySelector(".workstation-main-card")?.getBoundingClientRect().top ?? 0,
    }));

    expect(metrics.cardTop).toBeGreaterThan(0);
    expect(metrics.cardBottom).toBeLessThanOrEqual(metrics.viewportHeight - 16);
  });

  test("keeps host and join setup side panels beside the main form at desktop width", async ({ page }) => {
    await openProbe(page, "host");

    const hostMain = await getRect(page.locator(".workstation-grid > :first-child"));
    const hostSide = await getRect(page.locator(".workstation-side-panel"));
    expectRightColumn(hostMain, hostSide);

    await openProbe(page, "join");

    const joinMain = await getRect(page.locator(".workstation-grid > :first-child"));
    const joinSide = await getRect(page.locator(".join-preview-panel"));
    expectRightColumn(joinMain, joinSide);
  });

  test("keeps home resume and lobby ready-room content in usable desktop columns", async ({ page }) => {
    await openProbe(page, "home");

    const heroCard = await getRect(page.locator(".home-stage > :first-child"));
    const recoveryRail = await getRect(page.locator(".recovery-rail-card"));
    expectRightColumn(heroCard, recoveryRail);

    await openProbe(page, "lobby");

    const lobbyBrief = await getRect(page.locator(".lobby-brief-card"));
    const lobbyRoster = await getRect(page.locator(".lobby-roster-card"));
    expectRightColumn(lobbyBrief, lobbyRoster);

    const lobbyMetrics = await page.evaluate(() => ({
      viewportHeight: window.innerHeight,
      scrollHeight: document.documentElement.scrollHeight,
      cardBottom:
        document.querySelector(".workstation-main-card")?.getBoundingClientRect().bottom ?? 0,
    }));

    expect(lobbyMetrics.cardBottom).toBeLessThanOrEqual(lobbyMetrics.viewportHeight - 16);
    expect(lobbyMetrics.scrollHeight).toBeLessThanOrEqual(lobbyMetrics.viewportHeight);
  });

  test("keeps the home hero aligned to the left edge when no resume rail is present", async ({ page }) => {
    await openProbe(page, "home-empty");

    const stage = await getRect(page.locator(".home-stage"));
    const heroCard = await getRect(page.locator(".home-hero-card"));

    expect(Math.abs(heroCard.x - stage.x)).toBeLessThanOrEqual(4);
    expect(heroCard.width).toBeGreaterThan(stage.width * 0.9);
  });

  test("keeps history and completion support rails beside the primary content at desktop width", async ({ page }) => {
    await openProbe(page, "history");

    const historyPrimary = await getRect(page.locator(".history-primary-card"));
    const historySide = await getRect(page.locator(".history-side-stack"));
    expectRightColumn(historyPrimary, historySide);

    await openProbe(page, "complete");

    const completePrimary = await getRect(page.locator(".history-primary-card"));
    const completeSide = await getRect(page.locator(".history-side-stack"));
    expectRightColumn(completePrimary, completeSide);
  });

  test("keeps help in a two-column support grid at desktop width", async ({ page }) => {
    await openProbe(page, "help");

    const firstCard = await getRect(page.locator(".help-grid > :first-child"));
    const secondCard = await getRect(page.locator(".help-grid > :nth-child(2)"));
    expectRightColumn(firstCard, secondCard);
  });
});