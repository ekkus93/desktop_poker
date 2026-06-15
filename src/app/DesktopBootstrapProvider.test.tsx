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
  const actual =
    await vi.importActual<typeof import("../api/desktop")>("../api/desktop");

  return {
    ...actual,
    fetchBootstrapState: vi.fn(),
    subscribeBootstrap: vi.fn(),
  };
});

const mockedFetchBootstrapState = vi.mocked(fetchBootstrapState);
const mockedSubscribeBootstrap = vi.mocked(subscribeBootstrap);

function BootstrapProbe() {
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

describe("DesktopBootstrapProvider", () => {
  beforeEach(() => {
    mockedFetchBootstrapState.mockReset();
    mockedSubscribeBootstrap.mockReset();
  });

  it("loads bootstrap state and applies subscription updates", async () => {
    const initialBootstrap = createAppBootstrap({
      instanceId: "host-a",
      instanceLabel: "Host A",
    });
    const updatedBootstrap = createAppBootstrap({
      instanceId: "client-b",
      instanceLabel: "Client B",
    });
    const unsubscribe = vi.fn();
    let subscriptionHandler:
      | ((bootstrap: DesktopBootstrapState) => void)
      | undefined;

    mockedSubscribeBootstrap.mockImplementation(async (handler) => {
      subscriptionHandler = handler;
      return unsubscribe;
    });
    mockedFetchBootstrapState.mockResolvedValue(initialBootstrap);

    const renderResult = render(
      <DesktopBootstrapProvider>
        <BootstrapProbe />
      </DesktopBootstrapProvider>,
    );

    expect(await screen.findByText("Instance: host-a")).toBeTruthy();
    expect(screen.getByText("Loading: no")).toBeTruthy();

    await act(async () => {
      subscriptionHandler?.(updatedBootstrap);
    });

    expect(screen.getByText("Instance: client-b")).toBeTruthy();
    expect(screen.getByText("Label: Client B")).toBeTruthy();

    renderResult.unmount();
    expect(unsubscribe).toHaveBeenCalledTimes(1);
  });

  it("surfaces bootstrap load failures", async () => {
    mockedSubscribeBootstrap.mockResolvedValue(() => {});
    mockedFetchBootstrapState.mockRejectedValue(
      new Error("backend bootstrap offline"),
    );

    render(
      <DesktopBootstrapProvider>
        <BootstrapProbe />
      </DesktopBootstrapProvider>,
    );

    expect(
      await screen.findByText("Error: backend bootstrap offline"),
    ).toBeTruthy();
    expect(screen.getByText("Loading: no")).toBeTruthy();
    expect(screen.getByText("Instance: none")).toBeTruthy();
  });
});
