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

    expect(screen.queryByText("2/2 ready")).toBeNull();
    expect(screen.getByText("0/2 ready")).toBeTruthy();
    expect(screen.getByText("You: Waiting")).toBeTruthy();
    expect(screen.getByText("2 waiting")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Mark ready" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Host marks ready" })).toBeNull();
    expect(screen.getByText(/waiting on player readiness/i)).toBeTruthy();
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

    expect(screen.getByText(/pending seats use host-controlled ready toggles in debug mode/i)).toBeTruthy();

    expect(screen.getByText("Ready to start")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Start tournament" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Leave table" })).toBeTruthy();
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
    expect(screen.getByText("1/2 ready")).toBeTruthy();
  });
});