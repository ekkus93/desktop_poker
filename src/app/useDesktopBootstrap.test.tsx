import { render, screen } from "@testing-library/react";
import { act } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchBootstrapState,
  subscribeBootstrap,
  type DesktopBootstrapState,
} from "../api/desktop";
import { createAppBootstrap } from "../test/appIntegrationFixtures";
import { DesktopBootstrapProvider } from "./DesktopBootstrapProvider";
import { useDesktopBootstrap } from "./useDesktopBootstrap";

vi.mock("../api/desktop", async () => {
  const actual = await vi.importActual<typeof import("../api/desktop")>(
    "../api/desktop",
  );

  return {
    ...actual,
    fetchBootstrapState: vi.fn(),
    subscribeBootstrap: vi.fn(),
  };
});

const mockedFetchBootstrapState = vi.mocked(fetchBootstrapState);
const mockedSubscribeBootstrap = vi.mocked(subscribeBootstrap);

function BootstrapHookProbe() {
  const { bootstrap, error, loading } = useDesktopBootstrap();

  return (
    <div>
      <p>Loading: {loading ? "yes" : "no"}</p>
      <p>Error: {error ?? "none"}</p>
      <p>Instance: {bootstrap?.instanceId ?? "none"}</p>
      <p>Label: {bootstrap?.instanceLabel ?? "none"}</p>
    </div>
  );
}

describe("useDesktopBootstrap", () => {
  beforeEach(() => {
    mockedFetchBootstrapState.mockReset();
    mockedSubscribeBootstrap.mockReset();
  });

  it("fails clearly when used outside the provider", () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});

    expect(() => render(<BootstrapHookProbe />)).toThrow(
      "useDesktopBootstrap must be used inside the provider.",
    );

    consoleError.mockRestore();
  });

  it("exposes loading before bootstrap resolution and publishes the loaded bootstrap", async () => {
    let resolveBootstrap: ((value: DesktopBootstrapState) => void) | undefined;
    const bootstrapPromise = new Promise<DesktopBootstrapState>((resolve) => {
      resolveBootstrap = resolve;
    });

    mockedSubscribeBootstrap.mockResolvedValue(() => {});
    mockedFetchBootstrapState.mockReturnValue(bootstrapPromise);

    render(
      <DesktopBootstrapProvider>
        <BootstrapHookProbe />
      </DesktopBootstrapProvider>,
    );

    expect(screen.getByText("Loading: yes")).toBeTruthy();
    expect(screen.getByText("Instance: none")).toBeTruthy();

    await act(async () => {
      resolveBootstrap?.(
        createAppBootstrap({ instanceId: "host-a", instanceLabel: "Host A" }),
      );
      await bootstrapPromise;
    });

    expect(screen.getByText("Loading: no")).toBeTruthy();
    expect(screen.getByText("Instance: host-a")).toBeTruthy();
    expect(screen.getByText("Label: Host A")).toBeTruthy();
  });

  it("applies subscription updates and unsubscribes on unmount", async () => {
    const unsubscribe = vi.fn();
    let subscriptionHandler:
      | ((bootstrap: DesktopBootstrapState) => void)
      | undefined;

    mockedSubscribeBootstrap.mockImplementation(async (handler) => {
      subscriptionHandler = handler;
      return unsubscribe;
    });
    mockedFetchBootstrapState.mockResolvedValue(
      createAppBootstrap({ instanceId: "initial", instanceLabel: "Initial" }),
    );

    const renderResult = render(
      <DesktopBootstrapProvider>
        <BootstrapHookProbe />
      </DesktopBootstrapProvider>,
    );

    expect(await screen.findByText("Instance: initial")).toBeTruthy();

    await act(async () => {
      subscriptionHandler?.(
        createAppBootstrap({ instanceId: "updated", instanceLabel: "Updated" }),
      );
    });

    expect(screen.getByText("Instance: updated")).toBeTruthy();
    expect(screen.getByText("Label: Updated")).toBeTruthy();

    renderResult.unmount();
    expect(unsubscribe).toHaveBeenCalledTimes(1);
  });

  it("surfaces fetch failures through the hook state", async () => {
    mockedSubscribeBootstrap.mockResolvedValue(() => {});
    mockedFetchBootstrapState.mockRejectedValue(new Error("bootstrap offline"));

    render(
      <DesktopBootstrapProvider>
        <BootstrapHookProbe />
      </DesktopBootstrapProvider>,
    );

    expect(await screen.findByText("Error: bootstrap offline")).toBeTruthy();
    expect(screen.getByText("Loading: no")).toBeTruthy();
    expect(screen.getByText("Instance: none")).toBeTruthy();
  });
});