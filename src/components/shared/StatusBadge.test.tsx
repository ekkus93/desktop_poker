import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StatusBadge } from "./StatusBadge";

describe("StatusBadge", () => {
  it("renders children with the correct tone class", () => {
    render(<StatusBadge tone="success">Active</StatusBadge>);
    const badge = screen.getByText("Active").closest("span");
    expect(badge?.className).toContain("success");
  });

  it("renders an icon for each tone without exposing it to screen readers (A2)", () => {
    const { rerender } = render(<StatusBadge tone="success">Active</StatusBadge>);
    // Icon is aria-hidden — it does not appear as an accessible element
    let badge = screen.getByText("Active").closest("span")!;
    expect(badge.querySelector("[aria-hidden='true']")).toBeTruthy();

    rerender(<StatusBadge tone="warning">Slow</StatusBadge>);
    badge = screen.getByText("Slow").closest("span")!;
    expect(badge.querySelector("[aria-hidden='true']")).toBeTruthy();

    rerender(<StatusBadge tone="info">Waiting</StatusBadge>);
    badge = screen.getByText("Waiting").closest("span")!;
    expect(badge.querySelector("[aria-hidden='true']")).toBeTruthy();
  });

  it("renders no icon for accent tone", () => {
    render(<StatusBadge tone="accent">Level 2</StatusBadge>);
    const badge = screen.getByText("Level 2").closest("span")!;
    // accent has no icon defined in TONE_ICONS
    expect(badge.querySelector("[aria-hidden='true']")).toBeNull();
  });
});
