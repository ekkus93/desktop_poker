import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  createHistoryEntry,
  seedPersistedHandHistory,
} from "../test/persistenceFixtures";
import {
  initializeWindowStatePersistence,
  persistHandHistory,
  readPersistedHandHistory,
} from "./persistence";

const tauriWindowMock = vi.hoisted(() => {
  const state = {
    size: { width: 1280, height: 720 },
    position: { x: 80, y: 120 },
    maximized: false,
  };
  const listeners = {
    moved: new Set<() => void>(),
    resized: new Set<() => void>(),
  };

  const appWindow = {
    maximize: vi.fn(async () => {
      state.maximized = true;
    }),
    setSize: vi.fn(
      async (size: {
        width: number;
        height: number;
      }) => {
        state.size = { width: size.width, height: size.height };
      },
    ),
    setPosition: vi.fn(
      async (position: {
        x: number;
        y: number;
      }) => {
        state.position = { x: position.x, y: position.y };
      },
    ),
    innerSize: vi.fn(async () => state.size),
    outerPosition: vi.fn(async () => state.position),
    isMaximized: vi.fn(async () => state.maximized),
    onMoved: vi.fn(async (listener: () => void) => {
      listeners.moved.add(listener);
      return () => listeners.moved.delete(listener);
    }),
    onResized: vi.fn(async (listener: () => void) => {
      listeners.resized.add(listener);
      return () => listeners.resized.delete(listener);
    }),
  };

  return {
    appWindow,
    listeners,
    state,
    reset() {
      state.size = { width: 1280, height: 720 };
      state.position = { x: 80, y: 120 };
      state.maximized = false;
      listeners.moved.clear();
      listeners.resized.clear();
      appWindow.maximize.mockClear();
      appWindow.setSize.mockClear();
      appWindow.setPosition.mockClear();
      appWindow.innerSize.mockClear();
      appWindow.outerPosition.mockClear();
      appWindow.isMaximized.mockClear();
      appWindow.onMoved.mockClear();
      appWindow.onResized.mockClear();
    },
  };
});

vi.mock("@tauri-apps/api/window", () => {
  class LogicalSize {
    width: number;
    height: number;

    constructor(width: number, height: number) {
      this.width = width;
      this.height = height;
    }
  }

  class LogicalPosition {
    x: number;
    y: number;

    constructor(x: number, y: number) {
      this.x = x;
      this.y = y;
    }
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

function enableTauriWindowRuntime() {
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    configurable: true,
    value: {},
  });
}

describe("persistence", () => {
  beforeEach(() => {
    localStorage.clear();
    tauriWindowMock.reset();
    delete (window as typeof window & { __TAURI_INTERNALS__?: unknown })
      .__TAURI_INTERNALS__;
  });

  it("returns null when persisted hand history is missing or malformed", () => {
    expect(readPersistedHandHistory("desktop-poker:test")).toBeNull();

    localStorage.setItem(
      "desktop-poker:test:hand-history-summaries",
      "{not-json",
    );
    expect(readPersistedHandHistory("desktop-poker:test")).toBeNull();
  });

  it("persists deduplicated hand-history entries in descending hand order", () => {
    persistHandHistory("desktop-poker:test", [
      createHistoryEntry({ handNumber: 2, summary: "Older summary" }),
      createHistoryEntry({ handNumber: 4, summary: "Newest summary" }),
      createHistoryEntry({
        handNumber: 2,
        summary: "Updated summary for hand two",
      }),
    ]);

    expect(readPersistedHandHistory("desktop-poker:test")?.entries).toEqual([
      createHistoryEntry({ handNumber: 4, summary: "Newest summary" }),
      createHistoryEntry({
        handNumber: 2,
        summary: "Updated summary for hand two",
      }),
    ]);
  });

  it("writes the first entry, appends later entries, and can clear stale cached history", () => {
    persistHandHistory("desktop-poker:test", [
      createHistoryEntry({ handNumber: 1, summary: "First summary" }),
    ]);
    expect(readPersistedHandHistory("desktop-poker:test")?.entries).toEqual([
      createHistoryEntry({ handNumber: 1, summary: "First summary" }),
    ]);

    persistHandHistory("desktop-poker:test", [
      createHistoryEntry({ handNumber: 1, summary: "First summary" }),
      createHistoryEntry({ handNumber: 3, summary: "Third summary" }),
    ]);
    expect(readPersistedHandHistory("desktop-poker:test")?.entries).toEqual([
      createHistoryEntry({ handNumber: 3, summary: "Third summary" }),
      createHistoryEntry({ handNumber: 1, summary: "First summary" }),
    ]);

    persistHandHistory("desktop-poker:test", []);
    expect(readPersistedHandHistory("desktop-poker:test")).toEqual({
      updatedAtMs: expect.any(Number),
      entries: [],
    });
  });

  it("filters malformed persisted hand-history entries without crashing", () => {
    seedPersistedHandHistory("desktop-poker:test", [
      createHistoryEntry({ handNumber: 3, summary: "Valid entry" }),
      {
        handNumber: "bad-hand",
        summary: "Invalid entry",
      } as unknown as typeof createHistoryEntry,
    ] as never[]);

    expect(readPersistedHandHistory("desktop-poker:test")).toEqual({
      updatedAtMs: 1234,
      entries: [createHistoryEntry({ handNumber: 3, summary: "Valid entry" })],
    });
  });

  it("does nothing when the Tauri window runtime is unavailable", async () => {
    const cleanup = initializeWindowStatePersistence("desktop-poker:test");

    await flushWindowPersistence();
    cleanup();

    expect(tauriWindowMock.appWindow.setSize).not.toHaveBeenCalled();
    expect(
      localStorage.getItem("desktop-poker:test:window-state"),
    ).toBeNull();
  });

  it("restores and persists window bounds for the current namespace", async () => {
    enableTauriWindowRuntime();
    localStorage.setItem(
      "desktop-poker:test-a:window-state",
      JSON.stringify({
        width: 900,
        height: 700,
        x: 24,
        y: 48,
        maximized: false,
      }),
    );
    localStorage.setItem(
      "desktop-poker:test-b:window-state",
      JSON.stringify({
        width: 640,
        height: 480,
        x: 1,
        y: 2,
        maximized: false,
      }),
    );

    const cleanup = initializeWindowStatePersistence("desktop-poker:test-a");

    await flushWindowPersistence();

    expect(tauriWindowMock.appWindow.setSize).toHaveBeenCalledWith(
      expect.objectContaining({ width: 900, height: 700 }),
    );
    expect(tauriWindowMock.appWindow.setPosition).toHaveBeenCalledWith(
      expect.objectContaining({ x: 24, y: 48 }),
    );

    tauriWindowMock.state.size = { width: 1024, height: 768 };
    tauriWindowMock.state.position = { x: 100, y: 140 };
    tauriWindowMock.listeners.resized.forEach((listener) => listener());
    tauriWindowMock.listeners.moved.forEach((listener) => listener());
    await flushWindowPersistence();

    expect(
      JSON.parse(
        localStorage.getItem("desktop-poker:test-a:window-state") ?? "null",
      ),
    ).toEqual({
      width: 1024,
      height: 768,
      x: 100,
      y: 140,
      maximized: false,
    });
    expect(
      JSON.parse(
        localStorage.getItem("desktop-poker:test-b:window-state") ?? "null",
      ),
    ).toEqual({
      width: 640,
      height: 480,
      x: 1,
      y: 2,
      maximized: false,
    });

    cleanup();
  });

  it("persists maximized state transitions and ignores duplicate resize events", async () => {
    enableTauriWindowRuntime();

    initializeWindowStatePersistence("desktop-poker:test");
    await flushWindowPersistence();

    tauriWindowMock.state.maximized = true;
    tauriWindowMock.listeners.resized.forEach((listener) => listener());
    tauriWindowMock.listeners.resized.forEach((listener) => listener());
    await flushWindowPersistence();

    expect(
      JSON.parse(
        localStorage.getItem("desktop-poker:test:window-state") ?? "null",
      ),
    ).toEqual({
      width: 1280,
      height: 720,
      x: 80,
      y: 120,
      maximized: true,
    });
  });

  it("restores maximized windows without applying stale bounds", async () => {
    enableTauriWindowRuntime();
    localStorage.setItem(
      "desktop-poker:test:window-state",
      JSON.stringify({
        width: 900,
        height: 700,
        x: 24,
        y: 48,
        maximized: true,
      }),
    );

    initializeWindowStatePersistence("desktop-poker:test");

    await flushWindowPersistence();

    expect(tauriWindowMock.appWindow.maximize).toHaveBeenCalledTimes(1);
    expect(tauriWindowMock.appWindow.setSize).not.toHaveBeenCalled();
    expect(tauriWindowMock.appWindow.setPosition).not.toHaveBeenCalled();
  });

  it("ignores malformed and structurally invalid stored window state", async () => {
    enableTauriWindowRuntime();
    localStorage.setItem("desktop-poker:test:window-state", "{not-json");
    initializeWindowStatePersistence("desktop-poker:test");
    await flushWindowPersistence();

    expect(tauriWindowMock.appWindow.setSize).not.toHaveBeenCalled();
    expect(tauriWindowMock.appWindow.setPosition).not.toHaveBeenCalled();

    tauriWindowMock.reset();
    localStorage.setItem(
      "desktop-poker:test:window-state",
      JSON.stringify({
        width: 900,
        x: 30,
        y: 50,
        maximized: false,
      }),
    );
    initializeWindowStatePersistence("desktop-poker:test");
    await flushWindowPersistence();

    expect(tauriWindowMock.appWindow.setSize).not.toHaveBeenCalled();
    expect(tauriWindowMock.appWindow.setPosition).not.toHaveBeenCalled();

    tauriWindowMock.reset();
    localStorage.setItem(
      "desktop-poker:test:window-state",
      JSON.stringify({
        width: -1,
        height: "bad",
        x: 30,
        y: 50,
        maximized: false,
      }),
    );
    initializeWindowStatePersistence("desktop-poker:test");
    await flushWindowPersistence();

    expect(tauriWindowMock.appWindow.setSize).not.toHaveBeenCalled();
    expect(tauriWindowMock.appWindow.setPosition).not.toHaveBeenCalled();

    tauriWindowMock.reset();
    localStorage.setItem(
      "desktop-poker:test:window-state",
      JSON.stringify({
        width: 0,
        height: 720,
        x: 30,
        y: 50,
        maximized: false,
      }),
    );
    initializeWindowStatePersistence("desktop-poker:test");
    await flushWindowPersistence();

    expect(tauriWindowMock.appWindow.setSize).not.toHaveBeenCalled();
    expect(tauriWindowMock.appWindow.setPosition).not.toHaveBeenCalled();
  });

  it("removes window listeners during cleanup", async () => {
    enableTauriWindowRuntime();

    const cleanup = initializeWindowStatePersistence("desktop-poker:test");

    await flushWindowPersistence();
    expect(tauriWindowMock.listeners.moved.size).toBe(1);
    expect(tauriWindowMock.listeners.resized.size).toBe(1);

    cleanup();

    expect(tauriWindowMock.listeners.moved.size).toBe(0);
    expect(tauriWindowMock.listeners.resized.size).toBe(0);
  });

  it("reads the correct stored window state after switching instance identity", async () => {
    enableTauriWindowRuntime();
    localStorage.setItem(
      "desktop-poker:host-a:window-state",
      JSON.stringify({
        width: 900,
        height: 700,
        x: 24,
        y: 48,
        maximized: false,
      }),
    );
    localStorage.setItem(
      "desktop-poker:client-b:window-state",
      JSON.stringify({
        width: 640,
        height: 480,
        x: 12,
        y: 18,
        maximized: false,
      }),
    );

    const firstCleanup = initializeWindowStatePersistence("desktop-poker:host-a");
    await flushWindowPersistence();
    expect(tauriWindowMock.appWindow.setSize).toHaveBeenLastCalledWith(
      expect.objectContaining({ width: 900, height: 700 }),
    );
    firstCleanup();

    tauriWindowMock.reset();
    const secondCleanup = initializeWindowStatePersistence("desktop-poker:client-b");
    await flushWindowPersistence();
    expect(tauriWindowMock.appWindow.setSize).toHaveBeenLastCalledWith(
      expect.objectContaining({ width: 640, height: 480 }),
    );
    secondCleanup();
  });
});
