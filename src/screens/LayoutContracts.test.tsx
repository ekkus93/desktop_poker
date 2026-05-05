import fs from "node:fs";
import path from "node:path";
import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getTableView, resolveHostLanAddress, validateJoinPayloadInput } from "../api/desktop";
import { createParsedJoinPayload, createTableViewSnapshot } from "../test/appIntegrationFixtures";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { HomeScreen } from "./HomeScreen";
import { HostTournamentSetupScreen } from "./HostTournamentSetupScreen";
import { JoinTournamentScreen } from "./JoinTournamentScreen";
import { MainTableScreen } from "./MainTableScreen";
import { TournamentLobbyScreen } from "./TournamentLobbyScreen";

vi.mock("../api/desktop", async () => {
  const actual = await vi.importActual<typeof import("../api/desktop")>(
    "../api/desktop",
  );

  return {
    ...actual,
    resolveHostLanAddress: vi.fn(),
    validateJoinPayloadInput: vi.fn(),
    getTableView: vi.fn(),
  };
});

const mockedResolveHostLanAddress = vi.mocked(resolveHostLanAddress);
const mockedValidateJoinPayloadInput = vi.mocked(validateJoinPayloadInput);
const mockedGetTableView = vi.mocked(getTableView);

const appCss = fs.readFileSync(
  path.resolve(import.meta.dirname, "../App.css"),
  "utf8",
);

describe("Layout contracts", () => {
  beforeEach(() => {
    mockedResolveHostLanAddress.mockReset();
    mockedValidateJoinPayloadInput.mockReset();
    mockedGetTableView.mockReset();
    mockedResolveHostLanAddress.mockResolvedValue("192.168.1.10");
    mockedValidateJoinPayloadInput.mockResolvedValue(createParsedJoinPayload());
    mockedGetTableView.mockResolvedValue(createTableViewSnapshot());
  });

  it("keeps the host screen outer layout full-width and the split layout inside the card", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    const { container } = renderWithProviders(
      <HostTournamentSetupScreen bootstrap={bootstrap} />,
      { bootstrap, initialEntries: ["/host"] },
    );

    expect(await screen.findByRole("heading", { name: "Host Tournament Setup" })).toBeTruthy();

    const outerLayout = container.querySelector(".host-station-layout");
    expect(outerLayout?.children.length).toBe(1);

    const workstationGrid = container.querySelector(".host-station-layout .workstation-grid");
    expect(workstationGrid?.children.length).toBe(2);
  });

  it("keeps the join screen outer layout full-width and the split layout inside the card", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    const { container } = renderWithProviders(
      <JoinTournamentScreen bootstrap={bootstrap} />,
      { bootstrap, initialEntries: ["/join"] },
    );

    expect(await screen.findByRole("heading", { name: "Join Tournament" })).toBeTruthy();

    const outerLayout = container.querySelector(".join-station-layout");
    expect(outerLayout?.children.length).toBe(1);

    const workstationGrid = container.querySelector(".join-station-layout .workstation-grid");
    expect(workstationGrid?.children.length).toBe(2);
  });

  it("renders major pages inside a single visible screen shell without empty sibling panels", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    const homeRender = renderWithProviders(<HomeScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/"],
    });
    expect(await screen.findByRole("heading", { name: "Choose a table" })).toBeTruthy();
    expect(homeRender.container.querySelector(".home-stage")?.children.length).toBe(1);
    homeRender.unmount();

    const lobbyRender = renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });
    expect(await screen.findByRole("heading", { name: "Lobby" })).toBeTruthy();
    expect(lobbyRender.container.querySelector(".pregame-workstation")?.children.length).toBe(1);
    lobbyRender.unmount();

    const tableRender = renderWithProviders(<MainTableScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/table"],
    });
    expect(await screen.findByRole("heading", { name: "Main Table" })).toBeTruthy();
    expect(tableRender.container.querySelector(".table-screen-shell")?.children.length).toBeGreaterThan(0);
  });

  it("locks the outer host and join wrappers to a single column in CSS", () => {
    expect(appCss).toMatch(/\.host-station-layout\s*\{[\s\S]*?grid-template-columns:\s*minmax\(0, 1fr\);[\s\S]*?\}/);
    expect(appCss).toMatch(/\.join-station-layout\s*\{[\s\S]*?grid-template-columns:\s*minmax\(0, 1fr\);[\s\S]*?\}/);
    expect(appCss).toMatch(/\.workstation-grid\s*\{[\s\S]*?grid-template-columns:\s*minmax\(0, 1\.2fr\) minmax\(19rem, 0\.92fr\);[\s\S]*?\}/);
  });

  it("does not collapse the host and join workstation split until the narrower breakpoint", () => {
    expect(appCss).toMatch(/@media \(max-width: 1100px\)\s*\{[\s\S]*?\.workstation-grid\s*\{[\s\S]*?grid-template-columns:\s*1fr;[\s\S]*?\}[\s\S]*?\}/);

    const wideBreakpointBlock = appCss.match(/@media \(max-width: 1280px\)\s*\{([\s\S]*?)\n\}/);
    expect(wideBreakpointBlock?.[1] ?? "").not.toContain(".workstation-grid");
  });

  it("keeps the desktop multi-panel support and lobby grids intact until the narrower breakpoint", () => {
    expect(appCss).toMatch(/@media \(max-width: 1100px\)\s*\{[\s\S]*?\.home-stage\.has-recovery,[\s\S]*?\.lobby-workstation-grid,[\s\S]*?\.history-station-layout,[\s\S]*?\.complete-grid,[\s\S]*?\.recovery-grid,[\s\S]*?\.help-grid,[\s\S]*?\.workstation-grid\s*\{[\s\S]*?grid-template-columns:\s*1fr;[\s\S]*?\}[\s\S]*?\}/);

    const wideBreakpointBlock = appCss.match(/@media \(max-width: 1280px\)\s*\{([\s\S]*?)\n\}/);
    const wideBlock = wideBreakpointBlock?.[1] ?? "";
    expect(wideBlock).not.toContain(".home-stage.has-recovery");
    expect(wideBlock).not.toContain(".lobby-workstation-grid");
    expect(wideBlock).not.toContain(".history-station-layout");
    expect(wideBlock).not.toContain(".complete-grid");
    expect(wideBlock).not.toContain(".recovery-grid");
    expect(wideBlock).not.toContain(".help-grid");
  });

  it("keeps the landing topbar and screen header horizontal until the narrower breakpoint", () => {
    expect(appCss).toMatch(/@media \(max-width: 920px\)\s*\{[\s\S]*?\.topbar\s*\{[\s\S]*?grid-template-columns:\s*1fr;[\s\S]*?\}[\s\S]*?\.topbar-nav\s*\{[\s\S]*?justify-content:\s*start;[\s\S]*?\}[\s\S]*?\.topbar-meta\s*\{[\s\S]*?justify-items:\s*start;[\s\S]*?\}[\s\S]*?\.screen-header\s*\{[\s\S]*?flex-direction:\s*column;[\s\S]*?\}[\s\S]*?\}/);

    const wideBreakpointBlock = appCss.match(/@media \(max-width: 1280px\)\s*\{([\s\S]*?)\n\}/);
    const wideBlock = wideBreakpointBlock?.[1] ?? "";
    expect(wideBlock).not.toContain(".topbar {");
    expect(wideBlock).not.toContain(".topbar-nav {");
    expect(wideBlock).not.toContain(".topbar-meta {");
    expect(wideBlock).not.toContain(".screen-header {");
  });
});