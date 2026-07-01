import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach, vi } from "vitest";

(
  globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

type TauriGlobalWindow = Window & {
  __TAURI_INTERNALS__?: unknown;
  __TAURI__?: unknown;
};

function clearTauriTestGlobals() {
  const w = window as TauriGlobalWindow;
  delete w.__TAURI_INTERNALS__;
  delete w.__TAURI__;
}

afterEach(() => {
  cleanup();
  clearTauriTestGlobals();
  vi.restoreAllMocks();
});

beforeEach(() => {
  clearTauriTestGlobals();
  localStorage.clear();
  Object.defineProperty(window.navigator, "clipboard", {
    configurable: true,
    value: {
      writeText: vi.fn().mockResolvedValue(undefined),
    },
  });
});
