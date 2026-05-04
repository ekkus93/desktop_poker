import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach, vi } from "vitest";

globalThis.IS_REACT_ACT_ENVIRONMENT = true;

afterEach(() => {
  cleanup();
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
