import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getDebugState,
  launchAdditionalClientInstance,
  type DebugInspectorState,
  type HostRuntimeHealth,
} from "../../api/desktop";
import { createBootstrap, renderWithProviders } from "../../test/fixtures";
import { DebugPanel } from "./DebugPanel";

vi.mock("../../api/desktop", async () => {
  const actual =
    await vi.importActual<typeof import("../../api/desktop")>(
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

function baseHostHealth(
  overrides: Partial<HostRuntimeHealth> = {},
): HostRuntimeHealth {
  return {
    acceptErrorCount: 0,
    streamTimeoutErrorCount: 0,
    tickAdvanceErrorCount: 0,
    publishErrorCount: 0,
    stateLockErrorCount: 0,
    streamCloneErrorCount: 0,
    clientRegistryErrorCount: 0,
    reconnectMarkErrorCount: 0,
    snapshotSyncErrorCount: 0,
    pendingJoinLimitRejectionCount: 0,
    connectedClientLimitRejectionCount: 0,
    lastError: null,
    lastSuccessfulTickMs: null,
    lastSuccessfulPublishMs: null,
    ...overrides,
  };
}

function baseDebugState(
  overrides: Partial<DebugInspectorState> = {},
): DebugInspectorState {
  return {
    protocolLog: [],
    snapshotJson: "{}",
    currentSequence: 4,
    currentHandNumber: 2,
    actionWindowSummary: "You · call 20 · min 40 · max 200 · legal Fold, Call",
    launchHint:
      "Spawn another debug client with its own storage namespace, or attach a copied pkr1_ payload to exercise local multi-instance join handoff.",
    npcTiltLevels: {},
    lastLlmFallback: null,
    lastNpcActionError: null,
    hostRuntimeHealth: null,
    ...overrides,
  };
}

describe("DebugPanel", () => {
  beforeEach(() => {
    mockedGetDebugState.mockReset();
    mockedLaunchAdditionalClientInstance.mockReset();
    mockedGetDebugState.mockResolvedValue(baseDebugState());
    mockedLaunchAdditionalClientInstance.mockResolvedValue(
      "host-a-p100-client-1",
    );
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
        name: "Launch extra client with payload",
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
      await screen.findByText(
        /copied join payload for another local instance/i,
      ),
    ).toBeTruthy();
  });

  it("shows NPC tilt section when at least one NPC is tilted", async () => {
    mockedGetDebugState.mockResolvedValue(
      baseDebugState({
        npcTiltLevels: { "npc-seat-0": "mild", "npc-seat-1": "none" },
      }),
    );
    const bootstrap = createBootstrap();
    renderWithProviders(<DebugPanel asScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    const list = await screen.findByTestId("npc-tilt-list");
    expect(list).toBeTruthy();
    expect(list.textContent).toContain("npc-seat-0");
    expect(list.textContent).toContain("mild");
    // npc-seat-1 is "none" so should be filtered out.
    expect(list.textContent).not.toContain("npc-seat-1");
  });

  it("hides NPC tilt section when all NPCs are at none", async () => {
    mockedGetDebugState.mockResolvedValue(
      baseDebugState({
        npcTiltLevels: { "npc-seat-0": "none" },
      }),
    );
    const bootstrap = createBootstrap();
    renderWithProviders(<DebugPanel asScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    await screen.findByText("Rust backend module map");
    expect(screen.queryByTestId("npc-tilt-list")).toBeNull();
  });

  it("shows LLM fallback section when last_llm_fallback is set", async () => {
    mockedGetDebugState.mockResolvedValue(
      baseDebugState({
        lastLlmFallback:
          "npc-seat-1: ProviderNotConfigured (profile=sharp-pat)",
      }),
    );
    const bootstrap = createBootstrap();
    renderWithProviders(<DebugPanel asScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    const reason = await screen.findByTestId("llm-fallback-reason");
    expect(reason.textContent).toContain("ProviderNotConfigured");
    expect(reason.textContent).toContain("npc-seat-1");
  });

  it("hides LLM fallback section when last_llm_fallback is null", async () => {
    mockedGetDebugState.mockResolvedValue(
      baseDebugState({ lastLlmFallback: null }),
    );
    const bootstrap = createBootstrap();
    renderWithProviders(<DebugPanel asScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    await screen.findByText("Rust backend module map");
    expect(screen.queryByTestId("llm-fallback-section")).toBeNull();
  });

  it("shows structured NPC action error section when lastNpcActionError is set", async () => {
    mockedGetDebugState.mockResolvedValue(
      baseDebugState({
        lastNpcActionError: {
          playerId: "npc-seat-1",
          action: "Fold",
          reason: "staleWindow",
          message: "npc-seat-1: action window expired",
          handNumber: 3,
          sequence: 1,
          submitted: false,
          occurredAtMs: 1700000000000,
        },
      }),
    );
    const bootstrap = createBootstrap();
    renderWithProviders(<DebugPanel asScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    await screen.findByTestId("npc-action-error-section");
    const msg = screen.getByTestId("npc-action-error-message");
    expect(msg.textContent).toContain("action window expired");
    const reason = screen.getByTestId("npc-action-error-reason");
    expect(reason.textContent).toBe("staleWindow");
    const submitted = screen.getByTestId("npc-action-error-submitted");
    expect(submitted.textContent).toBe("no");
  });

  it("hides NPC action error section when lastNpcActionError is null", async () => {
    mockedGetDebugState.mockResolvedValue(
      baseDebugState({ lastNpcActionError: null }),
    );
    const bootstrap = createBootstrap();
    renderWithProviders(<DebugPanel asScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    await screen.findByText("Rust backend module map");
    expect(screen.queryByTestId("npc-action-error-section")).toBeNull();
  });

  it("renders all non-zero host runtime health counters", async () => {
    mockedGetDebugState.mockResolvedValue(
      baseDebugState({
        hostRuntimeHealth: baseHostHealth({
          acceptErrorCount: 1,
          tickAdvanceErrorCount: 2,
          publishErrorCount: 3,
          stateLockErrorCount: 4,
          streamTimeoutErrorCount: 5,
          streamCloneErrorCount: 6,
          clientRegistryErrorCount: 7,
          reconnectMarkErrorCount: 8,
          snapshotSyncErrorCount: 9,
          pendingJoinLimitRejectionCount: 10,
          connectedClientLimitRejectionCount: 11,
        }),
      }),
    );
    const bootstrap = createBootstrap();
    renderWithProviders(<DebugPanel asScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    await screen.findByTestId("host-runtime-health-section");
    expect(screen.getByText(/Accept errors:/i)).toBeTruthy();
    expect(screen.getByText(/Tick errors:/i)).toBeTruthy();
    expect(screen.getByText(/Publish errors:/i)).toBeTruthy();
    expect(screen.getByText(/Lock errors:/i)).toBeTruthy();
    expect(screen.getByText(/Stream timeout errors:/i)).toBeTruthy();
    expect(screen.getByText(/Stream clone errors:/i)).toBeTruthy();
    expect(screen.getByText(/Client registry errors:/i)).toBeTruthy();
    expect(screen.getByText(/Reconnect mark errors:/i)).toBeTruthy();
    expect(screen.getByText(/Snapshot sync errors:/i)).toBeTruthy();
    expect(screen.getByText(/Pending join limit rejections:/i)).toBeTruthy();
    expect(screen.getByText(/Connected client limit rejections:/i)).toBeTruthy();
  });

  it("hides host runtime health section when all counters are zero and no last error", async () => {
    mockedGetDebugState.mockResolvedValue(
      baseDebugState({ hostRuntimeHealth: baseHostHealth() }),
    );
    const bootstrap = createBootstrap();
    renderWithProviders(<DebugPanel asScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    await screen.findByText("Rust backend module map");
    expect(screen.queryByTestId("host-runtime-health-section")).toBeNull();
  });

  it("shows submitted=yes for rejected NPC action error", async () => {
    mockedGetDebugState.mockResolvedValue(
      baseDebugState({
        lastNpcActionError: {
          playerId: "npc-seat-1",
          action: "Call",
          reason: "rejected",
          message: "npc-seat-1: submit_action rejected",
          handNumber: 2,
          sequence: 2,
          submitted: true,
          occurredAtMs: 1700000001000,
        },
      }),
    );
    const bootstrap = createBootstrap();
    renderWithProviders(<DebugPanel asScreen bootstrap={bootstrap} />, {
      bootstrap,
    });

    await screen.findByTestId("npc-action-error-section");
    expect(screen.getByTestId("npc-action-error-reason").textContent).toBe(
      "rejected",
    );
    expect(screen.getByTestId("npc-action-error-submitted").textContent).toBe(
      "yes",
    );
  });
});
