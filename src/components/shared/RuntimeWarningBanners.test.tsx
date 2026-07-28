import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { RuntimeWarningBanners } from "./RuntimeWarningBanners";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("RuntimeWarningBanners", () => {
  beforeEach(() => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
    delete (window as Window & { __TAURI_INTERNALS__?: unknown })
      .__TAURI_INTERNALS__;
  });

  it("renders sanitized host warnings returned by the normal runtime command", async () => {
    mockedInvoke.mockResolvedValue([
      "One or more players missed a table update and may need to reconnect.",
    ]);

    render(<RuntimeWarningBanners />);

    expect(
      await screen.findByText(/players missed a table update/i),
    ).toBeTruthy();
    expect(mockedInvoke).toHaveBeenCalledWith("get_runtime_warnings");
  });

  it("surfaces warning-status command failures instead of hiding them", async () => {
    mockedInvoke.mockRejectedValue(new Error("host session lock poisoned"));

    render(<RuntimeWarningBanners />);

    expect(
      await screen.findByText(/runtime health status is unavailable/i),
    ).toBeTruthy();
    expect(screen.getByRole("alert")).toBeTruthy();
  });
});
