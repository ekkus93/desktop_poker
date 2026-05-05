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

function expectRightColumn(main: Rect, side: Rect) {
  expect(side.x).toBeGreaterThan(main.x + main.width * 0.55);
  expect(Math.abs(side.y - main.y)).toBeLessThanOrEqual(8);
}

test.describe("layout probe browser geometry", () => {
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

  test("keeps home resume and lobby action rail in separate desktop columns", async ({ page }) => {
    await openProbe(page, "home");

    const heroCard = await getRect(page.locator(".home-stage > :first-child"));
    const recoveryRail = await getRect(page.locator(".recovery-rail-card"));
    expectRightColumn(heroCard, recoveryRail);

    await openProbe(page, "lobby");

    const lobbySeatArea = await getRect(page.locator(".lobby-workstation-grid > :nth-child(2)"));
    const lobbyActionRail = await getRect(page.locator(".lobby-action-rail"));
    expectRightColumn(lobbySeatArea, lobbyActionRail);
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