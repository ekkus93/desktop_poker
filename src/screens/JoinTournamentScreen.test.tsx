import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  type JoinPayload,
  validateJoinPayloadInput,
} from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
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

const parsedPayload: JoinPayload = {
  payloadVersion: 1,
  hostAddress: "192.168.1.10",
  hostPort: 43818,
  tableId: "table-123",
  sessionEpoch: 42,
  hostSigningPublicKey: "pub-key",
  joinToken: "join-token",
  generatedAtMs: 99,
  tableName: "Friday Night",
};

describe("JoinTournamentScreen", () => {
  beforeEach(() => {
    mockedValidateJoinPayloadInput.mockReset();
  });

  it("prefers a deep-link payload over the stored draft", async () => {
    const bootstrap = createBootstrap({ launchJoinPayload: "pkr1_launch" });

    renderWithProviders(<JoinTournamentScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/join?payload=pkr1_link"],
    });

    await waitFor(() => {
      expect(
        (screen.getByLabelText("Paste payload") as HTMLTextAreaElement).value,
      ).toBe("pkr1_link");
    });
    expect(screen.getByText(/imported from a deep-link launch/i)).toBeTruthy();
  });

  it("shows validation failures from the Rust parser", async () => {
    const bootstrap = createBootstrap();
    mockedValidateJoinPayloadInput.mockRejectedValueOnce(
      new Error("invalid compact join payload"),
    );

    renderWithProviders(<JoinTournamentScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    fireEvent.change(screen.getByLabelText("Paste payload"), {
      target: { value: "pkr1_bad" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Validate payload" }));

    expect(await screen.findByText("invalid compact join payload")).toBeTruthy();
  });

  it("shows decoded payload details after validation", async () => {
    const bootstrap = createBootstrap();
    mockedValidateJoinPayloadInput.mockResolvedValueOnce(parsedPayload);

    renderWithProviders(<JoinTournamentScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    fireEvent.change(screen.getByLabelText("Paste payload"), {
      target: { value: "pkr1_good" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Connect" }));

    expect(await screen.findByText("Friday Night")).toBeTruthy();
    expect(screen.getByText(/next backend step is wiring this shell/i)).toBeTruthy();
  });
});
