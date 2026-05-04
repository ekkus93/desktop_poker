import { fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { resolveHostLanAddress } from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { ErrorStateScreen } from "./ErrorStateScreen";

vi.mock("../api/desktop", async () => {
  const actual = await vi.importActual<typeof import("../api/desktop")>(
    "../api/desktop",
  );

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

  it("surfaces an invalid launch payload as the primary state", async () => {
    const bootstrap = createBootstrap({ launchJoinPayloadError: "bad payload" });

    renderWithProviders(<ErrorStateScreen bootstrap={bootstrap} />, { bootstrap });

    expect((await screen.findAllByText(/the launch payload failed validation/i)).length).toBeGreaterThan(0);
    expect(screen.getAllByRole("link", { name: "Fix payload" }).length).toBeGreaterThan(0);
  });

  it("keeps the scenario picker available only for debug review", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: true });

    renderWithProviders(<ErrorStateScreen bootstrap={bootstrap} />, { bootstrap });

    expect((await screen.findAllByText(/the connection dropped/i)).length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole("button", { name: "Reconnect failed" }));
    expect((await screen.findAllByText(/could not restore the same session/i)).length).toBeGreaterThan(0);
  });

  it("exposes the reconnect recovery path with clear next actions", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: true });

    renderWithProviders(<ErrorStateScreen bootstrap={bootstrap} />, { bootstrap });

    fireEvent.click(screen.getByRole("button", { name: "Reconnected" }));

    expect((await screen.findAllByText(/you are back in the same session with the latest table state/i)).length).toBeGreaterThan(0);
    expect(screen.getAllByRole("link", { name: "Open table" }).length).toBeGreaterThan(0);
    expect(screen.getAllByRole("link", { name: "Open history" }).length).toBeGreaterThan(0);
  });
});