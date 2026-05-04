import { fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { resolveHostLanAddress } from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { createDefaultHostDraft, storageKey } from "../app/shell";
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

  it("shows all host game options by default and keeps them visible", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    expect(await screen.findByText(/ready on 192\.168\.1\.10/i)).toBeTruthy();
    expect(screen.getByLabelText(/starting stack/i)).toBeTruthy();
    expect(screen.getByLabelText(/blind preset/i)).toBeTruthy();
    expect(screen.getByLabelText(/turn timer/i)).toBeTruthy();
    expect(screen.getByLabelText(/host port/i)).toBeTruthy();
    expect(screen.getByText(/click the invite card or use the copy button below/i)).toBeTruthy();
  });

  it("copies invite details when the invite card is clicked", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    const inviteCard = await screen.findByRole("button", { name: /copy invite details/i });
    fireEvent.click(inviteCard);

    expect(writeText).toHaveBeenCalledWith(expect.stringContaining("Host endpoint:"));
    expect(await screen.findByText(/copied host share details\./i)).toBeTruthy();
  });

  it("opens advanced options when a persisted host draft is collapsed", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    const storedDraft = { ...createDefaultHostDraft(bootstrap), advancedOpen: false };
    localStorage.setItem(
      storageKey(bootstrap.storageNamespace, "host-draft"),
      JSON.stringify(storedDraft),
    );

    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    expect(await screen.findByText(/ready on 192\.168\.1\.10/i)).toBeTruthy();
    expect(screen.getByLabelText(/starting stack/i)).toBeTruthy();
    expect(screen.getByLabelText(/blind preset/i)).toBeTruthy();
    expect(screen.getByLabelText(/turn timer/i)).toBeTruthy();
    expect(screen.getByLabelText(/host port/i)).toBeTruthy();
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
    expect(screen.getByLabelText(/copy invite details/i)).toBeTruthy();
    expect(screen.getByText(/192\.168\.1\.10:43818/i)).toBeTruthy();
  });
});