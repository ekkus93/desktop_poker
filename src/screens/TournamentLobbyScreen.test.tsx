import { fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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

    expect(screen.getByText("4 open seats")).toBeTruthy();
    expect(screen.getByText("You: Waiting")).toBeTruthy();
    expect(screen.getByText("2 waiting")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Mark ready" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Host marks ready" })).toBeNull();
    expect(screen.getByText("Seat 1")).toBeTruthy();
    expect(screen.getByText("Seat 6")).toBeTruthy();
    expect(screen.getAllByText("Open seat").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "Start tournament" }).hasAttribute("disabled")).toBe(true);
  });

  it("promotes the start action only when the table is actually ready in debug shell mode", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: true });

    renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    fireEvent.click(screen.getByRole("button", { name: "Mark ready" }));
    fireEvent.click(screen.getByRole("button", { name: "Host marks ready" }));

    expect(screen.getByRole("link", { name: "Start tournament" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Leave table" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Host marked ready" })).toBeTruthy();
  });

  it("tracks the local ready badge without relying on display text matching", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false, instanceLabel: "host-alpha" });

    renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    expect(screen.getByText("You: Waiting")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Mark ready" }));

    expect(screen.getByText("You: Ready")).toBeTruthy();
    expect(screen.getByText("1 waiting")).toBeTruthy();
  });

  it("supports keyboard readiness toggles for the local player", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    const user = userEvent.setup();

    renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    const readyButton = screen.getByRole("button", { name: "Mark ready" });

    while (document.activeElement !== readyButton) {
      await user.tab();
    }

    expect(document.activeElement).toBe(readyButton);

    await user.keyboard("[Enter]");

    expect(screen.getByText("You: Ready")).toBeTruthy();
    expect(screen.getByText("1 waiting")).toBeTruthy();
  });
});