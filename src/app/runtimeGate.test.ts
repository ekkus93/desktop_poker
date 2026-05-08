import { describe, expect, it } from "vitest";
import { resolveLayoutProbeSurface } from "./runtimeGate";

describe("runtimeGate", () => {
  it("allows the layout probe only when explicitly enabled", () => {
    expect(resolveLayoutProbeSurface("?layout-probe=home", true)).toBe("home");
    expect(resolveLayoutProbeSurface("?layout-probe=home", false)).toBeNull();
  });

  it("returns null when the query parameter is absent", () => {
    expect(resolveLayoutProbeSurface("", true)).toBeNull();
  });
});
