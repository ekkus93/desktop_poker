import { fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  joinHostSession,
  type JoinPayload,
  validateJoinPayloadInput,
} from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { createParsedJoinPayload } from "../test/appIntegrationFixtures";
import { JoinTournamentScreen } from "./JoinTournamentScreen";

vi.mock("../api/desktop", async () => {
  const actual = await vi.importActual<typeof import("../api/desktop")>(
    "../api/desktop",
  );

  return {
    ...actual,
    joinHostSession: vi.fn(),
    resolveHostLanAddress: vi.fn().mockResolvedValue("192.168.1.10"),
    validateJoinPayloadInput: vi.fn(),
  };
});

const mockedJoinHostSession = vi.mocked(joinHostSession);
const mockedValidateJoinPayloadInput = vi.mocked(validateJoinPayloadInput);

const parsedPayload: JoinPayload = createParsedJoinPayload();

describe("JoinTournamentScreen", () => {
  beforeEach(() => {
    mockedJoinHostSession.mockReset();
    mockedJoinHostSession.mockResolvedValue({
      tournamentName: "Friday Night",
      tableName: "Main Table",
      tableId: "table-1",
      sessionEpoch: 9,
      hostAddress: "192.168.1.10",
      hostPort: 43818,
      localPlayerId: "player-test-instance",
      phase: "waitingForPlayers",
      activeSeatCount: 1,
      openSeatCount: 5,
      reconnecting: false,
      lastError: null,
      participants: [],
    });
    mockedValidateJoinPayloadInput.mockReset();
  });

  it("prefers a deep-link invite over the stored draft", async () => {
    const bootstrap = createBootstrap({ launchJoinPayload: "pkr1_launch" });
    mockedValidateJoinPayloadInput.mockResolvedValueOnce(parsedPayload);

    renderWithProviders(<JoinTournamentScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/join?payload=pkr1_link"],
    });

    await waitFor(() => {
      expect(
        (screen.getByLabelText("Invite") as HTMLTextAreaElement).value,
      ).toBe("pkr1_link");
    });
    expect(screen.getByText(/imported from a deep-link launch/i)).toBeTruthy();
    expect(await screen.findByRole("button", { name: "Continue to lobby" })).toBeTruthy();
  });

  it("joins immediately for a valid launch-attached invite", async () => {
    const bootstrap = createBootstrap({
      launchJoinPayload: "pkr1_launch",
      parsedLaunchJoinPayload: parsedPayload,
    });

    renderWithProviders(<JoinTournamentScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/join"],
    });

    expect(await screen.findByLabelText("Invite preview")).toBeTruthy();
    expect(await screen.findByText(/joining the attached invite now/i)).toBeTruthy();
    await waitFor(() => {
      expect(mockedJoinHostSession).toHaveBeenCalledWith({
        joinPayload: "pkr1_launch",
        displayName: "Player test-instance",
      });
    });
  });

  it("shows invite review failures from the Rust parser", async () => {
    const bootstrap = createBootstrap();
    mockedValidateJoinPayloadInput.mockRejectedValueOnce(
      new Error("invalid compact join payload"),
    );

    renderWithProviders(<JoinTournamentScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    fireEvent.change(screen.getByLabelText("Invite"), {
      target: { value: "pkr1_bad" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Check invite" }));

    expect(await screen.findByText("invalid compact join payload")).toBeTruthy();
    expect(screen.getByText(/fix the invite above before continuing to the lobby/i)).toBeTruthy();
  });

  it("explains when pasted host share details are not a compact invite", async () => {
    const bootstrap = createBootstrap();

    renderWithProviders(<JoinTournamentScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    fireEvent.change(screen.getByLabelText("Invite"), {
      target: {
        value: [
          "Tournament: Desktop Sit 'n Go host-a",
          "Host endpoint: 192.168.4.2:43818",
          "Capacity: 2 players · 1500 starting chips",
          "Blind preset: Standard (10 / 20 start)",
          "Turn timer: 30s",
          "Share these host details with the next player.",
          "This text is not a compact pkr1_ invite.",
        ].join("\n"),
      },
    });
    fireEvent.click(screen.getByRole("button", { name: "Check invite" }));

    expect(
      await screen.findByText(
        /that pasted text is host share details, not a compact pkr1_ invite/i,
      ),
    ).toBeTruthy();
    expect(mockedValidateJoinPayloadInput).not.toHaveBeenCalled();
  });

  it("shows decoded invite details after validation", async () => {
    const bootstrap = createBootstrap();
    mockedValidateJoinPayloadInput.mockResolvedValueOnce(parsedPayload);

    renderWithProviders(<JoinTournamentScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    fireEvent.change(screen.getByLabelText("Invite"), {
      target: { value: "pkr1_good" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Check invite" }));

    expect(await screen.findByLabelText("Invite preview")).toBeTruthy();
    expect(screen.getAllByText("Friday Night").length).toBeGreaterThan(0);
    expect(screen.getByText("Ready: 192.168.1.10:43818")).toBeTruthy();
    expect(
      screen.getByRole("button", { name: "Continue to lobby" }),
    ).toBeTruthy();
    expect(screen.getByText(/invite checked\. join when ready/i)).toBeTruthy();
  });

  it("joins the live host session before navigating to the lobby", async () => {
    const bootstrap = createBootstrap();
    mockedValidateJoinPayloadInput.mockResolvedValueOnce(parsedPayload);

    renderWithProviders(<JoinTournamentScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    fireEvent.change(screen.getByLabelText("Invite"), {
      target: { value: "pkr1_good" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Check invite" }));
    fireEvent.click(await screen.findByRole("button", { name: "Continue to lobby" }));

    await waitFor(() => {
      expect(mockedJoinHostSession).toHaveBeenCalledWith({
        joinPayload: "pkr1_good",
        displayName: "Player test-instance",
      });
    });
  });

  it("shows escape actions when auto-join from launch payload fails (B8)", async () => {
    const bootstrap = createBootstrap({
      launchJoinPayload: "pkr1_launch",
      parsedLaunchJoinPayload: createParsedJoinPayload(),
    });
    mockedJoinHostSession.mockRejectedValueOnce(new Error("Host rejected the connection"));

    renderWithProviders(<JoinTournamentScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/join"],
    });

    expect(await screen.findByText(/host rejected the connection/i)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Clear and enter a different invite" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Return home" })).toBeTruthy();
  });

  it("keeps keyboard focus moving through the join flow in a sane order", async () => {
    const bootstrap = createBootstrap();
    const user = userEvent.setup();
    mockedValidateJoinPayloadInput.mockResolvedValueOnce(parsedPayload);

    renderWithProviders(<JoinTournamentScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    const inviteField = screen.getByLabelText("Invite");
    const checkInviteButton = screen.getByRole("button", { name: "Check invite" });

    await user.tab();
    expect(document.activeElement).toBe(inviteField);

    await user.type(inviteField, "pkr1_good");
    await user.tab();
    expect(document.activeElement).toBe(checkInviteButton);

    await user.keyboard("[Enter]");

    const continueLink = await screen.findByRole("button", { name: "Continue to lobby" });
    await user.tab();
    expect(document.activeElement).toBe(continueLink);
  });
});
