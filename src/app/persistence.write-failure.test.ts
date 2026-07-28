import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createHistoryEntry } from "../test/persistenceFixtures";
import {
  initializeWindowStatePersistence,
  persistHandHistory,
} from "./persistence";

const tauriWindowMock = vi.hoisted(() => ({
  appWindow: {
    maximize: vi.fn(async () => {}),
    setSize: vi.fn(async () => {}),
    setPosition: vi.fn(async () => {}),
    innerSize: vi.fn(async () => ({ width: 1280, height: 720 })),
    outerPosition: vi.fn(async () => ({ x: 80, y: 120 })),
    isMaximized: vi.fn(async () => false),
    onMoved: vi.fn(async () => () => {}),
    onResized: vi.fn(async () => () => {}),
  },
}));

vi.mock("@tauri-apps/api/window", () => {
  class LogicalSize {
    constructor(
      public width: number,
      public height: number,
    ) {}
  }

  class LogicalPosition {
    constructor(
      public x: number,
      public y: number,
    ) {}
  }

  return {
    LogicalPosition,
    LogicalSize,
    getCurrentWindow: () => tauriWindowMock.appWindow,
  };
});

async function flushWindowPersistence() {
  await vi.dynamicImportSettled();
  await Promise.resolve();
  await Promise.resolve();
}

describe("persistence write failures", () => {
  beforeEach(() => {
    localStorage.clear();
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    delete (window as typeof window & { __TAURI_INTERNALS__?: unknown })
      .__TAURI_INTERNALS__;
  });

  it("reports cached hand-history write failures without throwing", () => {
    const onWriteFailure = vi.fn();
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new DOMException("quota exceeded", "QuotaExceededError");
    });

    expect(() =>
      persistHandHistory(
        "desktop-poker:test",
        [createHistoryEntry({ handNumber: 1 })],
        onWriteFailure,
      ),
    ).not.toThrow();

    expect(onWriteFailure).toHaveBeenCalledWith(
      expect.stringContaining("Failed to persist cached hand history"),
    );
  });

  it("reports window-state write failures without breaking initialization", async () => {
    const onWriteFailure = vi.fn();
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new DOMException("storage denied", "SecurityError");
    });

    const cleanup = initializeWindowStatePersistence(
      "desktop-poker:test",
      onWriteFailure,
    );
    await flushWindowPersistence();

    expect(onWriteFailure).toHaveBeenCalledWith(
      expect.stringContaining("Failed to persist window state"),
    );
    expect(tauriWindowMock.appWindow.onMoved).toHaveBeenCalled();
    expect(tauriWindowMock.appWindow.onResized).toHaveBeenCalled();

    cleanup();
  });
});
