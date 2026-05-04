import { fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { resolveHostLanAddress } from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { HostTournamentSetupScreen } from "./HostTournamentSetupScreen";

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

describe("HostTournamentSetupScreen", () => {
  beforeEach(() => {
    mockedResolveHostLanAddress.mockReset();
    mockedResolveHostLanAddress.mockResolvedValue("192.168.1.10");
  });

  it("keeps advanced settings collapsed until requested", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    expect(screen.queryByText(/resolved lan ip/i)).toBeNull();
    expect(await screen.findByText(/ready on 192\.168\.1\.10/i)).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: /show advanced settings/i }));

    expect(screen.getByText(/resolved lan ip/i)).toBeTruthy();
    expect(screen.getByText(/resolved lan ip/i)).toBeTruthy();
  });

  it("keeps sharing and the lobby transition obvious", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    expect(await screen.findByRole("button", { name: /copy invite/i })).toBeTruthy();
    expect(screen.getByRole("link", { name: /continue to lobby/i })).toBeTruthy();
    expect(screen.getByRole("heading", { level: 2, name: "Host Tournament Setup" })).toBeTruthy();
    expect(screen.getByLabelText("Invite card")).toBeTruthy();
    expect(screen.getByText(/192\.168\.1\.10:43818/i)).toBeTruthy();
  });
});