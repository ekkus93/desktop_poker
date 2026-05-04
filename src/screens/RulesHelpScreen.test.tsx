import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { RulesHelpScreen } from "./RulesHelpScreen";

describe("RulesHelpScreen", () => {
  it("keeps the rules and settings surface player-friendly", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<RulesHelpScreen bootstrap={bootstrap} />, { bootstrap });

    expect(screen.getByRole("heading", { name: "Game Help" })).toBeTruthy();
    expect(screen.getByText("How joining works")).toBeTruthy();
    expect(screen.getByRole("button", { name: /clear saved invites/i })).toBeTruthy();

    expect(screen.queryByText(/profile id/i)).toBeNull();
    expect(screen.queryByText(/instance label/i)).toBeNull();
    expect(screen.queryByText(/debug tools/i)).toBeNull();
    expect(screen.queryByText(/payload/i)).toBeNull();
  });
});