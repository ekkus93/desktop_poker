import { describe, expect, it } from "vitest";
import {
  buildQuickSizes,
  clampRaiseAmount,
  defaultRaiseAmount,
  isWithinRaiseBounds,
} from "./mainTableRaise";
import type { TableActionTrayView } from "../api/desktop";

function sampleTray(
  overrides?: Partial<TableActionTrayView>,
): TableActionTrayView {
  return {
    ownerLabel: "You",
    checkOrCallLabel: "Call",
    betOrRaiseLabel: "Raise",
    callAmount: 100,
    currentBet: 100,
    potTotal: 400,
    minRaiseTo: 200,
    maxRaiseTo: 1000,
    deadlineEpochMs: 99999,
    legalActions: ["Call", "Raise", "Fold"],
    ...overrides,
  };
}

// T10.1 — clampRaiseAmount

describe("clampRaiseAmount", () => {
  it("clamps below minRaiseTo to minRaiseTo", () => {
    const tray = sampleTray();
    expect(clampRaiseAmount(100, tray)).toBe(200);
  });

  it("clamps above maxRaiseTo to maxRaiseTo", () => {
    const tray = sampleTray();
    expect(clampRaiseAmount(1500, tray)).toBe(1000);
  });

  it("returns amount unchanged when within bounds", () => {
    const tray = sampleTray();
    expect(clampRaiseAmount(500, tray)).toBe(500);
  });

  it("returns amount unchanged when minRaiseTo is null", () => {
    const tray = sampleTray({ minRaiseTo: null });
    expect(clampRaiseAmount(50, tray)).toBe(50);
  });

  it("returns amount unchanged when maxRaiseTo is null", () => {
    const tray = sampleTray({ maxRaiseTo: null });
    expect(clampRaiseAmount(9999, tray)).toBe(9999);
  });
});

// T10.2 — buildQuickSizes

describe("buildQuickSizes", () => {
  it("returns four labelled sizes in correct order", () => {
    const sizes = buildQuickSizes(sampleTray());
    expect(sizes).toHaveLength(4);
    expect(sizes[0]).toEqual({ label: "Min", amount: 200 });
    expect(sizes[1]).toEqual({ label: "1/2 Pot", amount: 200 }); // half-pot=200 clamped to min=200
    expect(sizes[2]).toEqual({ label: "Pot", amount: 400 });
    expect(sizes[3]).toEqual({ label: "Max", amount: 1000 });
  });

  it("returns empty when actionTray is undefined", () => {
    expect(buildQuickSizes(undefined)).toEqual([]);
  });

  it("returns empty when minRaiseTo is null", () => {
    expect(buildQuickSizes(sampleTray({ minRaiseTo: null }))).toEqual([]);
  });

  it("returns empty when maxRaiseTo is null", () => {
    expect(buildQuickSizes(sampleTray({ maxRaiseTo: null }))).toEqual([]);
  });

  it("clamps half-pot to maxRaiseTo when half-pot exceeds max", () => {
    const sizes = buildQuickSizes(
      sampleTray({ potTotal: 2400, maxRaiseTo: 1000 }),
    );
    const halfPot = sizes.find((s) => s.label === "1/2 Pot");
    expect(halfPot?.amount).toBe(1000);
  });
});

// T10.3 — defaultRaiseAmount

describe("defaultRaiseAmount", () => {
  it("returns minRaiseTo when available", () => {
    expect(defaultRaiseAmount(sampleTray())).toBe(200);
  });

  it("returns maxRaiseTo when minRaiseTo is null", () => {
    expect(defaultRaiseAmount(sampleTray({ minRaiseTo: null }))).toBe(1000);
  });

  it("returns currentBet when both min and max are null", () => {
    expect(
      defaultRaiseAmount(sampleTray({ minRaiseTo: null, maxRaiseTo: null })),
    ).toBe(100); // currentBet
  });
});

// T10.4 — isWithinRaiseBounds

describe("isWithinRaiseBounds", () => {
  it("returns true at inclusive lower bound", () => {
    expect(isWithinRaiseBounds(200, sampleTray())).toBe(true);
  });

  it("returns true at inclusive upper bound", () => {
    expect(isWithinRaiseBounds(1000, sampleTray())).toBe(true);
  });

  it("returns false below lower bound", () => {
    expect(isWithinRaiseBounds(199, sampleTray())).toBe(false);
  });

  it("returns false above upper bound", () => {
    expect(isWithinRaiseBounds(1001, sampleTray())).toBe(false);
  });

  it("returns false when minRaiseTo is null", () => {
    expect(isWithinRaiseBounds(500, sampleTray({ minRaiseTo: null }))).toBe(
      false,
    );
  });

  it("returns false when maxRaiseTo is null", () => {
    expect(isWithinRaiseBounds(500, sampleTray({ maxRaiseTo: null }))).toBe(
      false,
    );
  });
});
