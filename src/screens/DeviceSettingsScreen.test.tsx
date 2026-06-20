import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  clearLlmProviderConfig,
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
    clearLlmProviderConfig: vi.fn(),
    saveNonSecretProviderSettings: vi.fn(),
  };
});

const mockedGetLlmProviderConfig = vi.mocked(getLlmProviderConfig);
const mockedSetLlmProviderConfig = vi.mocked(setLlmProviderConfig);
const mockedClearLlmProviderConfig = vi.mocked(clearLlmProviderConfig);
const mockedSaveNonSecretProviderSettings = vi.mocked(
  saveNonSecretProviderSettings,
);

describe("DeviceSettingsScreen", () => {
  beforeEach(() => {
    mockedGetLlmProviderConfig.mockReset();
    mockedSetLlmProviderConfig.mockReset();
    mockedClearLlmProviderConfig.mockReset();
    mockedSaveNonSecretProviderSettings.mockReset();
    mockedGetLlmProviderConfig.mockResolvedValue(null);
    mockedSetLlmProviderConfig.mockResolvedValue(undefined);
    mockedClearLlmProviderConfig.mockResolvedValue(undefined);
    mockedSaveNonSecretProviderSettings.mockResolvedValue(undefined);
  });

  it("keeps local device controls separate from gameplay help", () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    renderWithProviders(<DeviceSettingsScreen />, { bootstrap });
    expect(screen.getByRole("heading", { name: "Settings" })).toBeTruthy();
    expect(screen.getByLabelText("Display name")).toBeTruthy();
    expect(
      screen.getByRole("button", { name: /reset host setup/i }),
    ).toBeTruthy();
    expect(
      screen.getByRole("button", { name: /clear saved invites/i }),
    ).toBeTruthy();
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

  it("shows Not configured status when llmApiKeyConfigured is false", () => {
    const bootstrap = createBootstrap({ llmApiKeyConfigured: false });
    renderWithProviders(<DeviceSettingsScreen />, { bootstrap });
    expect(screen.getByText("Not configured")).toBeTruthy();
  });

  it("shows Configured status with provider type when llmApiKeyConfigured is true", () => {
    const bootstrap = createBootstrap({
      llmApiKeyConfigured: true,
      llmProviderType: "anthropic",
    });
    renderWithProviders(<DeviceSettingsScreen />, { bootstrap });
    expect(screen.getByText(/configured/i)).toBeTruthy();
  });

  it("calls setLlmProviderConfig with anthropic provider when Save is clicked", async () => {
    const bootstrap = createBootstrap({ llmApiKeyConfigured: false });
    renderWithProviders(<DeviceSettingsScreen />, { bootstrap });

    fireEvent.change(screen.getByLabelText("API key"), {
      target: { value: "sk-ant-test123" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(mockedSetLlmProviderConfig).toHaveBeenCalledWith(
        expect.objectContaining({
          provider: "anthropic",
          apiKey: "sk-ant-test123",
        }),
      );
    });
  });

  it("calls setLlmProviderConfig with ollama provider (no api key required)", async () => {
    const bootstrap = createBootstrap({ llmApiKeyConfigured: false });
    renderWithProviders(<DeviceSettingsScreen />, { bootstrap });

    fireEvent.change(screen.getByLabelText("LLM provider"), {
      target: { value: "ollama" },
    });

    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(mockedSetLlmProviderConfig).toHaveBeenCalledWith(
        expect.objectContaining({ provider: "ollama" }),
      );
    });
  });

  it("calls clearLlmProviderConfig when Clear is clicked", async () => {
    const bootstrap = createBootstrap({
      llmApiKeyConfigured: true,
      llmProviderType: "anthropic",
    });
    renderWithProviders(<DeviceSettingsScreen />, { bootstrap });

    fireEvent.click(screen.getByRole("button", { name: "Clear" }));

    await waitFor(() => {
      expect(mockedClearLlmProviderConfig).toHaveBeenCalled();
    });
  });

  it("shows plaintext-storage warning when an API-key-requiring provider is selected", () => {
    const bootstrap = createBootstrap({ llmApiKeyConfigured: false });
    renderWithProviders(<DeviceSettingsScreen />, { bootstrap });

    // Anthropic is selected by default — warning should be visible.
    expect(
      screen.getByText(/release builds store api keys in the os keychain/i),
    ).toBeTruthy();
  });

  it("does not show plaintext-storage warning for providers that do not require API keys", () => {
    const bootstrap = createBootstrap({ llmApiKeyConfigured: false });
    renderWithProviders(<DeviceSettingsScreen />, { bootstrap });

    // Switch to Ollama, which needs no key.
    fireEvent.change(screen.getByLabelText("LLM provider"), {
      target: { value: "ollama" },
    });

    expect(screen.queryByText(/release builds store api keys in the os keychain/i)).toBeNull();
  });

  it("shows error when setLlmProviderConfig fails", async () => {
    mockedSetLlmProviderConfig.mockRejectedValue(
      new Error("Connection refused"),
    );
    const bootstrap = createBootstrap({ llmApiKeyConfigured: false });
    renderWithProviders(<DeviceSettingsScreen />, { bootstrap });

    // Use ollama (no api key needed) so Save is not disabled.
    fireEvent.change(screen.getByLabelText("LLM provider"), {
      target: { value: "ollama" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(screen.getByText("Connection refused")).toBeTruthy();
    });
  });
});
