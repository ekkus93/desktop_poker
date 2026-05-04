import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { TournamentLobbyScreen } from "./TournamentLobbyScreen";

describe("TournamentLobbyScreen", () => {
  it("keeps ready and waiting states easy to distinguish", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    expect(screen.queryByText("2 of 2 seated players are ready")).toBeNull();
    expect(screen.getByText("0 of 2 seated players are ready")).toBeTruthy();
    expect(screen.getByText("You still need to mark ready")).toBeTruthy();
    expect(screen.getByText("Waiting on the table")).toBeTruthy();
    expect(screen.getAllByRole("button", { name: "Mark ready" }).length).toBeGreaterThanOrEqual(2);
    expect(screen.getByRole("button", { name: "Start tournament" }).hasAttribute("disabled")).toBe(true);
  });

  it("promotes the start action only when the table is actually ready", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    const readyButtons = screen.getAllByRole("button", { name: "Mark ready" });
    fireEvent.click(readyButtons[0]);
    fireEvent.click(readyButtons[1]);

    expect(screen.getByText("Everyone is ready for the first hand")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Start tournament" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Leave table" })).toBeTruthy();
  });
});