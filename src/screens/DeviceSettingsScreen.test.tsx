import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { clearLlmApiKey, setLlmApiKey } from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { DeviceSettingsScreen } from "./DeviceSettingsScreen";

vi.mock("../api/desktop", async () => {
  const actual = await vi.importActual<typeof import("../api/desktop")>("../api/desktop");
  return {
    ...actual,
    setLlmApiKey: vi.fn(),
    clearLlmApiKey: vi.fn(),
  };
});

const mockedSetLlmApiKey = vi.mocked(setLlmApiKey);
const mockedClearLlmApiKey = vi.mocked(clearLlmApiKey);

describe("DeviceSettingsScreen", () => {
  beforeEach(() => {
    mockedSetLlmApiKey.mockReset();
    mockedClearLlmApiKey.mockReset();
    mockedSetLlmApiKey.mockResolvedValue(undefined);
    mockedClearLlmApiKey.mockResolvedValue(undefined);
  });

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

  it("shows No key set status when llmApiKeyConfigured is false", () => {
    const bootstrap = createBootstrap({ llmApiKeyConfigured: false });
    renderWithProviders(<DeviceSettingsScreen />, { bootstrap });
    expect(screen.getByText("No key set")).toBeTruthy();
  });

  it("shows Key configured status when llmApiKeyConfigured is true", () => {
    const bootstrap = createBootstrap({ llmApiKeyConfigured: true });
    renderWithProviders(<DeviceSettingsScreen />, { bootstrap });
    expect(screen.getByText("Key configured")).toBeTruthy();
  });

  it("calls setLlmApiKey when Save key is clicked with a value", async () => {
    const bootstrap = createBootstrap({ llmApiKeyConfigured: false });
    renderWithProviders(<DeviceSettingsScreen />, { bootstrap });

    fireEvent.change(screen.getByPlaceholderText("sk-ant-..."), {
      target: { value: "sk-ant-test123" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save key" }));

    await waitFor(() => {
      expect(mockedSetLlmApiKey).toHaveBeenCalledWith("sk-ant-test123");
    });
  });

  it("calls clearLlmApiKey when Clear key is clicked", async () => {
    const bootstrap = createBootstrap({ llmApiKeyConfigured: true });
    renderWithProviders(<DeviceSettingsScreen />, { bootstrap });

    fireEvent.click(screen.getByRole("button", { name: "Clear key" }));

    await waitFor(() => {
      expect(mockedClearLlmApiKey).toHaveBeenCalled();
    });
  });

  it("shows error when setLlmApiKey fails", async () => {
    mockedSetLlmApiKey.mockRejectedValue(new Error("Invalid key format"));
    const bootstrap = createBootstrap({ llmApiKeyConfigured: false });
    renderWithProviders(<DeviceSettingsScreen />, { bootstrap });

    fireEvent.change(screen.getByPlaceholderText("sk-ant-..."), {
      target: { value: "bad-key" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save key" }));

    await waitFor(() => {
      expect(screen.getByText("Invalid key format")).toBeTruthy();
    });
  });
});
