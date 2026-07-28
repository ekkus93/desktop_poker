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

  it("opens one modal reset confirmation and disables both launch actions", () => {
    renderSettings();

    const resetButton = screen.getByRole("button", {
      name: "Reset host setup",
    });
    resetButton.focus();
    fireEvent.click(resetButton);

    expect(
      screen.getByRole("dialog", { name: "Reset saved host setup?" }),
    ).toBeTruthy();
    expect(screen.getByRole("button", { name: "Confirm reset" })).toBe(
      document.activeElement,
    );
    expect((resetButton as HTMLButtonElement).disabled).toBe(true);
    expect(
      (
        screen.getByRole("button", {
          name: /clear saved invites/i,
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
    expect(
      screen.queryByRole("dialog", { name: "Clear saved invitations?" }),
    ).toBeNull();
  });

  it("opens one modal clear confirmation and disables both launch actions", () => {
    renderSettings();

    const clearButton = screen.getByRole("button", {
      name: /clear saved invites/i,
    });
    clearButton.focus();
    fireEvent.click(clearButton);

    expect(
      screen.getByRole("dialog", { name: "Clear saved invitations?" }),
    ).toBeTruthy();
    expect(screen.getByRole("button", { name: "Confirm clear" })).toBe(
      document.activeElement,
    );
    expect((clearButton as HTMLButtonElement).disabled).toBe(true);
    expect(
      (
        screen.getByRole("button", {
          name: "Reset host setup",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
    expect(
      screen.queryByRole("dialog", { name: "Reset saved host setup?" }),
    ).toBeNull();
  });

  it("cancel closes the confirmation, re-enables actions, and restores focus", () => {
    renderSettings();

    const resetButton = screen.getByRole("button", {
      name: "Reset host setup",
    });
    resetButton.focus();
    fireEvent.click(resetButton);
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(screen.queryByRole("dialog")).toBeNull();
    expect((resetButton as HTMLButtonElement).disabled).toBe(false);
    expect(
      (
        screen.getByRole("button", {
          name: /clear saved invites/i,
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(false);
    expect(document.activeElement).toBe(resetButton);
  });
});
