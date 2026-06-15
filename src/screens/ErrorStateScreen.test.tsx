import { fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { resolveHostLanAddress } from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { ErrorStateScreen } from "./ErrorStateScreen";

vi.mock("../api/desktop", async () => {
  const actual =
    await vi.importActual<typeof import("../api/desktop")>("../api/desktop");

  return {
    ...actual,
    resolveHostLanAddress: vi.fn(),
  };
});

const mockedResolveHostLanAddress = vi.mocked(resolveHostLanAddress);

describe("ErrorStateScreen", () => {
  beforeEach(() => {
    mockedResolveHostLanAddress.mockReset();
    mockedResolveHostLanAddress.mockResolvedValue("192.168.1.10");
  });

  it("surfaces an invalid launch invite as the primary state", async () => {
    const bootstrap = createBootstrap({
      launchJoinPayloadError: "bad payload",
    });

    renderWithProviders(<ErrorStateScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    expect(
      (await screen.findAllByText(/invite error: bad payload/i)).length,
    ).toBeGreaterThan(0);
    expect(
      screen.getAllByRole("link", { name: "Fix invite" }).length,
    ).toBeGreaterThan(0);
  });

  it("surfaces unreadable persisted startup data as a recovery state", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    localStorage.setItem(
      `${bootstrap.storageNamespace}:display-name`,
      "{bad-json",
    );

    renderWithProviders(<ErrorStateScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    expect(
      (await screen.findAllByText(/local data reset/i)).length,
    ).toBeGreaterThan(0);
    expect(
      screen.getByText(
        /some saved local preferences were unreadable and were reset to safe defaults/i,
      ),
    ).toBeTruthy();
    expect(screen.getByRole("link", { name: "Return home" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Open settings" })).toBeTruthy();
  });

  it("keeps scenario controls out of the normal player flow", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<ErrorStateScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    expect(
      (await screen.findAllByText(/reconnecting to the table/i)).length,
    ).toBeGreaterThan(0);
    expect(screen.queryByText("Scenario picker")).toBeNull();
    expect(
      screen.queryByRole("button", { name: "Reconnect failed" }),
    ).toBeNull();
  });

  it("keeps the scenario picker available only for debug review", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: true });

    renderWithProviders(<ErrorStateScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    expect(
      (await screen.findAllByText(/reconnecting to the table/i)).length,
    ).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole("button", { name: "Reconnect failed" }));
    expect(
      (await screen.findAllByText(/could not restore this session/i)).length,
    ).toBeGreaterThan(0);
  });

  it("exposes the reconnect recovery path with clear next actions", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: true });

    renderWithProviders(<ErrorStateScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    fireEvent.click(screen.getByRole("button", { name: "Reconnected" }));

    expect(
      (await screen.findAllByText(/back at the table/i)).length,
    ).toBeGreaterThan(0);
    expect(
      screen.getAllByRole("link", { name: "Open table" }).length,
    ).toBeGreaterThan(0);
    expect(
      screen.getAllByRole("link", { name: "Open history" }).length,
    ).toBeGreaterThan(0);
  });
});
