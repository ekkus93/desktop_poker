import { fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
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
    resolveHostLanAddress: vi.fn().mockResolvedValue("192.168.1.10"),
    validateJoinPayloadInput: vi.fn(),
  };
});

const mockedValidateJoinPayloadInput = vi.mocked(validateJoinPayloadInput);

const parsedPayload: JoinPayload = createParsedJoinPayload();

describe("JoinTournamentScreen", () => {
  beforeEach(() => {
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
    expect(await screen.findByRole("link", { name: "Continue to lobby" })).toBeTruthy();
  });

  it("shows the continue action for a valid launch-attached invite", async () => {
    const bootstrap = createBootstrap({
      launchJoinPayload: "pkr1_launch",
      parsedLaunchJoinPayload: parsedPayload,
    });

    renderWithProviders(<JoinTournamentScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/join"],
    });

    expect(await screen.findByLabelText("Invite preview")).toBeTruthy();
    expect(screen.getByText(/invite already attached to this launch/i)).toBeTruthy();
    expect(screen.getByRole("link", { name: "Continue to lobby" })).toBeTruthy();
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
      screen.getByRole("link", { name: "Continue to lobby" }),
    ).toBeTruthy();
    expect(screen.getByText(/invite checked\. continue when ready/i)).toBeTruthy();
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

    const continueLink = await screen.findByRole("link", { name: "Continue to lobby" });
    await user.tab();
    expect(document.activeElement).toBe(continueLink);
  });
});
