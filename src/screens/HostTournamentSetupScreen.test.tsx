import { fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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
    expect(screen.getByRole("region", { name: /host share summary/i })).toBeTruthy();
  });

  it("copies invite details when the copy button is clicked", async () => {
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

    fireEvent.click(await screen.findByRole("button", { name: /copy invite/i }));

    expect(writeText).toHaveBeenCalledWith(expect.stringContaining("Host endpoint:"));
    expect(await screen.findByText(/copied host share details\./i)).toBeTruthy();
  });

  it("supports keyboard activation on the copy button", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    const writeText = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    const copyButton = await screen.findByRole("button", { name: /copy invite/i });
    copyButton.focus();
    await user.keyboard("[Enter]");

    expect(writeText).toHaveBeenCalledWith(expect.stringContaining("Host endpoint:"));
    expect(await screen.findByText(/copied host share details\./i)).toBeTruthy();
  });

  it("blocks lobby continuation until hosting is ready", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    mockedResolveHostLanAddress.mockRejectedValueOnce(new Error("No reachable LAN IP"));

    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    expect((await screen.findAllByText(/no reachable lan ip/i)).length).toBeGreaterThan(0);
    expect(screen.queryByRole("link", { name: /continue to lobby/i })).toBeNull();
    expect(screen.getByRole("button", { name: /continue to lobby/i }).hasAttribute("disabled")).toBe(true);
    expect(screen.getByText(/resolve the lan address before continuing to the lobby/i)).toBeTruthy();
    expect(screen.getByText(/invite copy is unavailable until the lan address issue is fixed/i)).toBeTruthy();
  });

  it("shows a manual share fallback when clipboard copy fails", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    const writeText = vi.fn().mockRejectedValue(new Error("Clipboard denied"));
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    fireEvent.click(await screen.findByRole("button", { name: /copy invite/i }));

    expect(await screen.findByText(/copy failed\. share the invite details manually\./i)).toBeTruthy();
    expect(screen.getByDisplayValue(/host endpoint: 192\.168\.1\.10:43818/i)).toBeTruthy();
  });

  it("keeps critical setup options visible for legacy persisted host drafts", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    localStorage.setItem(
      storageKey(bootstrap.storageNamespace, "host-draft"),
      JSON.stringify({
        ...createDefaultHostDraft(bootstrap),
        advancedOpen: false,
      }),
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
    expect(screen.getByRole("region", { name: /host share summary/i })).toBeTruthy();
    expect(screen.getByText(/192\.168\.1\.10:43818/i)).toBeTruthy();
  });
});