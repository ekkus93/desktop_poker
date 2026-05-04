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

    expect(screen.getByText("Your seat is waiting")).toBeTruthy();
    expect(screen.getByText("The table is still waiting")).toBeTruthy();
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

    expect(screen.getByText("The table can start now")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Start tournament" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Leave table" })).toBeTruthy();
  });
});