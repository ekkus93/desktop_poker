import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach, vi } from "vitest";

(
  globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

afterEach(() => {
  cleanup();
  // Prevent fake Tauri runtime globals set by one test from leaking into the next.
  delete (window as typeof window & { __TAURI_INTERNALS__?: unknown })
    .__TAURI_INTERNALS__;
  delete (window as typeof window & { __TAURI__?: unknown }).__TAURI__;
});

beforeEach(() => {
  localStorage.clear();
  Object.defineProperty(window.navigator, "clipboard", {
    configurable: true,
    value: {
      writeText: vi.fn().mockResolvedValue(undefined),
    },
  });
});
