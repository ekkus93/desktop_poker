import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { RulesHelpScreen } from "./RulesHelpScreen";

describe("RulesHelpScreen", () => {
  it("keeps gameplay help separate from local device settings", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<RulesHelpScreen />, { bootstrap });

    expect(screen.getByRole("heading", { name: "Help" })).toBeTruthy();
    expect(screen.getByText("Host flow")).toBeTruthy();
    expect(screen.getByText("Join flow")).toBeTruthy();
    expect(screen.getByText("Observers and history")).toBeTruthy();
    expect(screen.queryByLabelText(/display name/i)).toBeNull();

    expect(screen.queryByText(/profile id/i)).toBeNull();
    expect(screen.queryByText(/instance label/i)).toBeNull();
    expect(screen.queryByText(/debug tools/i)).toBeNull();
    expect(screen.queryByText(/payload/i)).toBeNull();
  });
});