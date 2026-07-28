import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach, vi } from "vitest";

(
  globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

type TauriGlobalWindow = Window & {
  __TAURI_INTERNALS__?: unknown;
  __TAURI__?: unknown;
};

function installCompatibleRequestShim() {
  const NativeRequest = globalThis.Request;
  if (!NativeRequest) {
    return;
  }

  function CompatibleRequest(
    input: RequestInfo | URL,
    init?: RequestInit,
  ): Request {
    try {
      return new NativeRequest(input, init);
    } catch (error) {
      const message = error instanceof Error ? error.message : "";
      if (!init?.signal || !message.includes("Expected signal")) {
        throw error;
      }

      const compatibleInit: RequestInit = { ...init };
      delete compatibleInit.signal;
      return new NativeRequest(input, compatibleInit);
    }
  }

  Object.setPrototypeOf(CompatibleRequest, NativeRequest);
  CompatibleRequest.prototype = NativeRequest.prototype;
  globalThis.Request = CompatibleRequest as unknown as typeof Request;
}

function clearTauriTestGlobals() {
  const w = window as TauriGlobalWindow;
  delete w.__TAURI_INTERNALS__;
  delete w.__TAURI__;
}

installCompatibleRequestShim();

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
