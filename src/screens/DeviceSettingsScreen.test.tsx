import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { DeviceSettingsScreen } from "./DeviceSettingsScreen";

describe("DeviceSettingsScreen", () => {
  it("keeps local device controls separate from gameplay help", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<DeviceSettingsScreen />, { bootstrap });

    expect(screen.getByRole("heading", { name: "Settings" })).toBeTruthy();
    expect(screen.getByLabelText("Display name")).toBeTruthy();
    expect(screen.getByRole("button", { name: /reset host setup/i })).toBeTruthy();
    expect(screen.getByRole("button", { name: /clear saved invites/i })).toBeTruthy();
    expect(screen.queryByText("Table basics")).toBeNull();
  });

  it("updates the stored local display name", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });

    renderWithProviders(<DeviceSettingsScreen />, { bootstrap });

    fireEvent.change(screen.getByLabelText("Display name"), {
      target: { value: "Maya" },
    });

    expect(screen.getByDisplayValue("Maya")).toBeTruthy();
  });
});