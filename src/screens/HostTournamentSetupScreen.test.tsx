import { fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  addNpcPlayers,
  getHostSessionStatus,
  listNpcProfiles,
  resolveHostLanAddress,
  startHostSession,
} from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { createDefaultHostDraft, storageKey } from "../app/shell";
import { HostTournamentSetupScreen } from "./HostTournamentSetupScreen";

vi.mock("../api/desktop", async () => {
  const actual = await vi.importActual<typeof import("../api/desktop")>(
    "../api/desktop",
  );

  return {
    ...actual,
    addNpcPlayers: vi.fn(),
    getHostSessionStatus: vi.fn(),
    listNpcProfiles: vi.fn(),
    resolveHostLanAddress: vi.fn(),
    startHostSession: vi.fn(),
  };
});

const mockedAddNpcPlayers = vi.mocked(addNpcPlayers);
const mockedGetHostSessionStatus = vi.mocked(getHostSessionStatus);
const mockedListNpcProfiles = vi.mocked(listNpcProfiles);
const mockedResolveHostLanAddress = vi.mocked(resolveHostLanAddress);
const mockedStartHostSession = vi.mocked(startHostSession);

function sampleHostSessionStatus() {
  return {
    tournamentName: "Desktop Sit 'n Go test-instance",
    tableName: "Main Table",
    tableId: "table-1",
    sessionEpoch: 42,
    advertisedHost: "192.168.1.10",
    hostPort: 43818,
    invite: "pkr1_generated_invite",
    phase: "waitingForPlayers",
    activeSeatCount: 1,
    openSeatCount: 5,
    participants: [
      {
        playerId: "local-player",
        displayName: "Player test-instance",
        seatIndex: 0,
        isHost: true,
        isReady: false,
        connectionState: "connected",
        participantState: "seated",
      },
    ],
  };
}

describe("HostTournamentSetupScreen", () => {
  beforeEach(() => {
    mockedAddNpcPlayers.mockReset();
    mockedAddNpcPlayers.mockResolvedValue(sampleHostSessionStatus());
    mockedGetHostSessionStatus.mockReset();
    mockedGetHostSessionStatus.mockResolvedValue(null);
    mockedListNpcProfiles.mockReset();
    mockedListNpcProfiles.mockResolvedValue([]);
    mockedResolveHostLanAddress.mockReset();
    mockedResolveHostLanAddress.mockResolvedValue("192.168.1.10");
    mockedStartHostSession.mockReset();
    mockedStartHostSession.mockResolvedValue(sampleHostSessionStatus());
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
    expect(screen.getByRole("option", { name: /normal · small blind 10 · big blind 20/i })).toBeTruthy();
    expect(screen.getByRole("option", { name: /fast · small blind 10 · big blind 20/i })).toBeTruthy();
    expect(screen.getByRole("option", { name: /slow · small blind 10 · big blind 20/i })).toBeTruthy();
    expect(screen.queryByRole("option", { name: /standard/i })).toBeNull();
    expect(screen.queryByRole("option", { name: /turbo/i })).toBeNull();
    expect(screen.queryByRole("option", { name: /deep stack/i })).toBeNull();
    expect(screen.getByLabelText(/turn timer/i)).toBeTruthy();
    expect(screen.getByLabelText(/host port/i)).toBeTruthy();
    expect(screen.getByRole("region", { name: /host share summary/i })).toBeTruthy();
  });

  it("starts hosting and copies the live invite when requested", async () => {
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

    fireEvent.click(await screen.findByRole("button", { name: /start hosting/i }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /copy invite/i }).hasAttribute("disabled")).toBe(false);
    });
    fireEvent.click(screen.getByRole("button", { name: /copy invite/i }));

    await waitFor(() => {
      expect(mockedStartHostSession).toHaveBeenCalledWith({
        hostAddress: "192.168.1.10",
        hostPort: 43818,
        tournamentName: "Desktop Sit 'n Go test-instance",
        maxPlayers: 6,
        startingStack: 1500,
        blindPresetId: "normal",
        turnTimerSeconds: 30,
        displayName: "Player test-instance",
      });
      expect(writeText).toHaveBeenCalledWith("pkr1_generated_invite");
    });
    expect(await screen.findByText(/copied invite\./i)).toBeTruthy();
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

    fireEvent.click(await screen.findByRole("button", { name: /start hosting/i }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /copy invite/i }).hasAttribute("disabled")).toBe(false);
    });
    const copyButton = screen.getByRole("button", { name: /copy invite/i });
    copyButton.focus();
    await user.keyboard("[Enter]");

    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith("pkr1_generated_invite");
    });
    expect(await screen.findByText(/copied invite\./i)).toBeTruthy();
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

    fireEvent.click(await screen.findByRole("button", { name: /start hosting/i }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /copy invite/i }).hasAttribute("disabled")).toBe(false);
    });
    fireEvent.click(screen.getByRole("button", { name: /copy invite/i }));

    expect(await screen.findByText(/copy failed\. share the invite manually\./i)).toBeTruthy();
    expect(screen.getByDisplayValue("pkr1_generated_invite")).toBeTruthy();
  });

  it("surfaces host bind failures without unlocking lobby progression", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    mockedStartHostSession.mockRejectedValueOnce(new Error("Address already in use"));

    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    fireEvent.click(await screen.findByRole("button", { name: /start hosting/i }));

    expect(await screen.findByText("Address already in use")).toBeTruthy();
    expect(screen.queryByRole("link", { name: /continue to lobby/i })).toBeNull();
    expect(screen.getByRole("button", { name: /continue to lobby/i }).hasAttribute("disabled")).toBe(true);
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

    fireEvent.click(await screen.findByRole("button", { name: /start hosting/i }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /copy invite/i }).hasAttribute("disabled")).toBe(false);
    });

    expect(screen.getByRole("button", { name: /copy invite/i })).toBeTruthy();
    expect(await screen.findByRole("link", { name: /continue to lobby/i })).toBeTruthy();
    expect(screen.getByRole("heading", { level: 2, name: "Host Tournament Setup" })).toBeTruthy();
    expect(screen.getByRole("region", { name: /host share summary/i })).toBeTruthy();
    expect(screen.getByText(/192\.168\.1\.10:43818/i)).toBeTruthy();
  });

  it("disables Start hosting and shows an error when tournament name is blank (F1)", async () => {
    mockedResolveHostLanAddress.mockResolvedValue("192.168.1.10");
    mockedGetHostSessionStatus.mockResolvedValue(null);

    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    // Clear the tournament name
    const nameInput = await screen.findByLabelText("Tournament name");
    fireEvent.change(nameInput, { target: { value: "" } });
    fireEvent.blur(nameInput);

    expect(screen.getByText("Name is required.")).toBeTruthy();
    expect(screen.getByRole("button", { name: /start hosting/i }).hasAttribute("disabled")).toBe(true);
  });

  it("shows port validation feedback when an out-of-range port is entered (F2)", async () => {
    mockedResolveHostLanAddress.mockResolvedValue("192.168.1.10");
    mockedGetHostSessionStatus.mockResolvedValue(null);

    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    const portInput = await screen.findByLabelText("Host port");
    // The onChange handler checks the raw value before clamping
    fireEvent.change(portInput, { target: { value: "99999" } });

    expect(screen.getByText("Port must be between 1 and 65535.")).toBeTruthy();

    fireEvent.change(portInput, { target: { value: "43818" } });

    expect(screen.queryByText("Port must be between 1 and 65535.")).toBeNull();
  });

  it("disables the Start hosting button while hosting is starting (E3)", async () => {
    mockedResolveHostLanAddress.mockResolvedValue("192.168.1.10");
    mockedGetHostSessionStatus.mockResolvedValue(null);

    let resolveStart!: (value: ReturnType<typeof sampleHostSessionStatus>) => void;
    mockedStartHostSession.mockImplementation(
      () => new Promise((resolve) => { resolveStart = resolve; }),
    );

    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    fireEvent.click(await screen.findByRole("button", { name: /start hosting/i }));

    // Button should show "Starting…" and be disabled while the promise is pending
    expect(await screen.findByRole("button", { name: /starting/i })).toBeTruthy();
    expect(screen.getByRole("button", { name: /starting/i }).hasAttribute("disabled")).toBe(true);

    resolveStart(sampleHostSessionStatus());

    await waitFor(() => {
      expect(screen.queryByRole("button", { name: /starting/i })).toBeNull();
    });
  });

  it("hides NPC style selector when npcCount is 0", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    expect(await screen.findByLabelText(/npc players/i)).toBeTruthy();
    expect(screen.queryByLabelText(/npc style/i)).toBeNull();
  });

  it("shows NPC style selector when npcCount is set to 1 or more", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    const npcSelect = await screen.findByLabelText(/npc players/i);
    fireEvent.change(npcSelect, { target: { value: "2" } });

    expect(screen.getByLabelText(/npc style/i)).toBeTruthy();
  });

  it("calls addNpcPlayers with correct payload after startHostSession succeeds when npcCount > 0", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    const npcSelect = await screen.findByLabelText(/npc players/i);
    fireEvent.change(npcSelect, { target: { value: "2" } });

    const styleSelect = screen.getByLabelText(/npc style/i);
    fireEvent.change(styleSelect, { target: { value: "conservative" } });

    fireEvent.click(screen.getByRole("button", { name: /start hosting/i }));

    await waitFor(() => {
      expect(mockedAddNpcPlayers).toHaveBeenCalledWith({
        npcs: [
          { displayName: "Bot Alpha", style: "conservative", profileId: null },
          { displayName: "Bot Beta", style: "conservative", profileId: null },
        ],
      });
    });
  });

  it("does not call addNpcPlayers when npcCount is 0", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    fireEvent.click(await screen.findByRole("button", { name: /start hosting/i }));

    await waitFor(() => {
      expect(mockedStartHostSession).toHaveBeenCalled();
    });

    expect(mockedAddNpcPlayers).not.toHaveBeenCalled();
  });

  it("hides profile select when llmApiKeyConfigured is false", async () => {
    const bootstrap = createBootstrap({ llmApiKeyConfigured: false });
    localStorage.setItem(
      storageKey(bootstrap.storageNamespace, "host-draft"),
      JSON.stringify({ ...createDefaultHostDraft(bootstrap), npcCount: 2 }),
    );
    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, { bootstrap });
    await screen.findByRole("button", { name: /start hosting/i });
    expect(screen.queryByLabelText("AI profile")).toBeNull();
    expect(screen.getByText(/add a claude api key/i)).toBeTruthy();
  });

  it("shows profile select when llmApiKeyConfigured is true and npcCount > 0", async () => {
    mockedListNpcProfiles.mockResolvedValue([
      { id: "aggressive-alice", name: "Aggressive Alice", style: "loose-aggressive", skill: "intermediate", description: "", opponentTendencies: null, tiltBehaviour: null },
    ]);
    const bootstrap = createBootstrap({ llmApiKeyConfigured: true });
    localStorage.setItem(
      storageKey(bootstrap.storageNamespace, "host-draft"),
      JSON.stringify({ ...createDefaultHostDraft(bootstrap), npcCount: 1 }),
    );
    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, { bootstrap });
    await screen.findByRole("button", { name: /start hosting/i });
    expect(await screen.findByLabelText("AI profile")).toBeTruthy();
    expect(screen.getByText("Aggressive Alice")).toBeTruthy();
  });
});
