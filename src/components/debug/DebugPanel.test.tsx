import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getDebugState,
  launchAdditionalClientInstance,
} from "../../api/desktop";
import { createBootstrap, renderWithProviders } from "../../test/fixtures";
import { DebugPanel } from "./DebugPanel";

vi.mock("../../api/desktop", async () => {
  const actual = await vi.importActual<typeof import("../../api/desktop")>(
    "../../api/desktop",
  );

  return {
    ...actual,
    getDebugState: vi.fn(),
    launchAdditionalClientInstance: vi.fn(),
  };
});

const mockedGetDebugState = vi.mocked(getDebugState);
const mockedLaunchAdditionalClientInstance = vi.mocked(
  launchAdditionalClientInstance,
);
const clipboardWriteText = vi.fn();

describe("DebugPanel", () => {
  beforeEach(() => {
    mockedGetDebugState.mockReset();
    mockedLaunchAdditionalClientInstance.mockReset();
    mockedGetDebugState.mockResolvedValue({
      protocolLog: [],
      snapshotJson: "{}",
      currentSequence: 4,
      currentHandNumber: 2,
      actionWindowSummary: "You · call 20 · min 40 · max 200 · legal Fold, Call",
      launchHint:
        "Spawn another debug client with its own storage namespace, or attach a copied pkr1_ payload to exercise local multi-instance join handoff.",
    });
    mockedLaunchAdditionalClientInstance.mockResolvedValue("host-a-p100-client-1");
    clipboardWriteText.mockReset();
    clipboardWriteText.mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: clipboardWriteText,
      },
    });
    localStorage.clear();
  });

  it("shows instance isolation metadata and can launch another debug client with a payload", async () => {
    const bootstrap = createBootstrap({
      instanceId: "host-a",
      instanceLabel: "Host A",
      storageNamespace: "desktop-poker:host-a",
      sessionIdentity: "desktop-session:host-a",
      reconnectNamespace: "desktop-reconnect:host-a",
      launchJoinPayload: "pkr1_host_payload",
    });

    renderWithProviders(<DebugPanel asScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    expect(await screen.findByText("Host A")).toBeTruthy();
    expect(screen.getByText("desktop-poker:host-a")).toBeTruthy();
    expect(screen.getByText("desktop-session:host-a")).toBeTruthy();
    expect(screen.getByText("desktop-reconnect:host-a")).toBeTruthy();

    fireEvent.click(
      screen.getByRole("button", {
        name: "Launch extra debug client with payload",
      }),
    );

    await waitFor(() => {
      expect(clipboardWriteText).toHaveBeenCalledWith("pkr1_host_payload");
    });
    expect(mockedLaunchAdditionalClientInstance).toHaveBeenCalledWith(
      "pkr1_host_payload",
    );
    expect(
      await screen.findByText(/with the copied join payload attached/i),
    ).toBeTruthy();
  });

  it("copies the current payload draft without launching", async () => {
    const bootstrap = createBootstrap({
      launchJoinPayload: "pkr1_copy_me",
    });

    renderWithProviders(<DebugPanel asScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    fireEvent.click(screen.getByRole("button", { name: "Copy payload" }));

    await waitFor(() => {
      expect(clipboardWriteText).toHaveBeenCalledWith("pkr1_copy_me");
    });
    expect(mockedLaunchAdditionalClientInstance).not.toHaveBeenCalled();
    expect(
      await screen.findByText(/copied join payload for another local instance/i),
    ).toBeTruthy();
  });
});
