import { fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getLlmProviderConfig,
  saveNonSecretProviderSettings,
  setLlmProviderConfig,
} from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { DeviceSettingsScreen } from "./DeviceSettingsScreen";

vi.mock("../api/desktop", async () => {
  const actual =
    await vi.importActual<typeof import("../api/desktop")>("../api/desktop");
  return {
    ...actual,
    getLlmProviderConfig: vi.fn(),
    setLlmProviderConfig: vi.fn(),
    saveNonSecretProviderSettings: vi.fn(),
  };
});

const mockedGetLlmProviderConfig = vi.mocked(getLlmProviderConfig);
const mockedSetLlmProviderConfig = vi.mocked(setLlmProviderConfig);
const mockedSaveNonSecretProviderSettings = vi.mocked(
  saveNonSecretProviderSettings,
);

function renderSettings() {
  renderWithProviders(<DeviceSettingsScreen />, {
    bootstrap: createBootstrap({ debugToolsEnabled: false }),
  });
}

describe("DeviceSettingsScreen destructive confirmation exclusivity", () => {
  beforeEach(() => {
    localStorage.clear();
    mockedGetLlmProviderConfig.mockReset();
    mockedGetLlmProviderConfig.mockResolvedValue(null);
    mockedSetLlmProviderConfig.mockReset();
    mockedSetLlmProviderConfig.mockResolvedValue(undefined);
    mockedSaveNonSecretProviderSettings.mockReset();
    mockedSaveNonSecretProviderSettings.mockResolvedValue(undefined);
  });

  it("shows only reset confirmation and disables clear while reset is pending", () => {
    renderSettings();

    fireEvent.click(screen.getByRole("button", { name: "Reset host setup" }));

    expect(screen.getByRole("button", { name: "Confirm reset" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Confirm clear" })).toBeNull();
    const clearButton = screen.getByRole("button", {
      name: /clear saved invites/i,
    }) as HTMLButtonElement;
    expect(clearButton.disabled).toBe(true);
  });

  it("shows only clear confirmation and disables reset while clear is pending", () => {
    renderSettings();

    fireEvent.click(
      screen.getByRole("button", { name: /clear saved invites/i }),
    );

    expect(screen.getByRole("button", { name: "Confirm clear" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Confirm reset" })).toBeNull();
    const resetButton = screen.getByRole("button", {
      name: "Reset host setup",
    }) as HTMLButtonElement;
    expect(resetButton.disabled).toBe(true);
  });

  it("cancel closes the active confirmation and re-enables both actions", () => {
    renderSettings();

    fireEvent.click(screen.getByRole("button", { name: "Reset host setup" }));
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(screen.queryByRole("button", { name: "Confirm reset" })).toBeNull();
    expect(
      (
        screen.getByRole("button", {
          name: "Reset host setup",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(false);
    expect(
      (
        screen.getByRole("button", {
          name: /clear saved invites/i,
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(false);
  });
});
