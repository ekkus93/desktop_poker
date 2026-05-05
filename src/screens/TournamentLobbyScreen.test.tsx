import { fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { storageKey } from "../app/shell";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { TournamentLobbyScreen } from "./TournamentLobbyScreen";

describe("TournamentLobbyScreen", () => {
  it("keeps ready and waiting states easy to distinguish", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    expect(screen.getByText("Desktop Sit 'n Go test-instance")).toBeTruthy();
    expect(screen.getByText("5 open seats")).toBeTruthy();
    expect(screen.getByText("You: Waiting")).toBeTruthy();
    expect(screen.getByText("1 waiting")).toBeTruthy();
    expect(screen.getByRole("button", { name: "I'm ready" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Mark seat 2 ready" })).toBeNull();
    expect(screen.getByText("Seat 1")).toBeTruthy();
    expect(screen.getByText("Seat 6")).toBeTruthy();
    expect(screen.getAllByText("Open seat").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "Start tournament" }).hasAttribute("disabled")).toBe(true);
  });

  it("renders all 10 seats for a 10-player tournament and keeps the tournament name visible", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false, storageNamespace: "desktop-poker:ten-seat-lobby" });
    localStorage.setItem(
      storageKey(bootstrap.storageNamespace, "host-draft"),
      JSON.stringify({ tournamentName: "Friday Finals", maxPlayers: 10 }),
    );

    const { container } = renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    expect(screen.getByText("Friday Finals")).toBeTruthy();
    expect(screen.getByText("10 seats")).toBeTruthy();
    expect(screen.getByText("Seat 10")).toBeTruthy();
    expect(container.querySelector(".lobby-seat-grid")?.children.length).toBe(10);
  });

  it("keeps remote ready controls out of the normal lobby even in debug builds", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: true });

    renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    fireEvent.click(screen.getByRole("button", { name: "I'm ready" }));

    expect(screen.getByRole("button", { name: "Start tournament" }).hasAttribute("disabled")).toBe(true);
    expect(screen.getByRole("button", { name: "Close table" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Mark seat 2 ready" })).toBeNull();
  });

  it("tracks the local ready badge without relying on display text matching", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false, instanceLabel: "host-alpha" });

    renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    expect(screen.getByText("You: Waiting")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "I'm ready" }));

    expect(screen.getByText("You: Ready")).toBeTruthy();
    expect(screen.getByText("0 waiting")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Undo ready" })).toBeTruthy();
  });

  it("supports keyboard readiness toggles for the local player", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    const user = userEvent.setup();

    renderWithProviders(<TournamentLobbyScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/lobby"],
    });

    const readyButton = screen.getByRole("button", { name: "I'm ready" });

    while (document.activeElement !== readyButton) {
      await user.tab();
    }

    expect(document.activeElement).toBe(readyButton);

    await user.keyboard("[Enter]");

    expect(screen.getByText("You: Ready")).toBeTruthy();
    expect(screen.getByText("0 waiting")).toBeTruthy();
  });
});