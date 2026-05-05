import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ScreenShell } from "./ScreenShell";

describe("ScreenShell", () => {
  it("renders the title, lead copy, badges, and extra class name when provided", () => {
    const { container } = render(
      <ScreenShell
        title="Ready Room"
        lead="All players must confirm readiness before start."
        badges={["45s timer", "2 participants"]}
        className="ready-room-shell"
      >
        <div>Child content</div>
      </ScreenShell>,
    );

    expect(screen.getByText("Desktop Poker")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Ready Room" })).toBeTruthy();
    expect(
      screen.getByText("All players must confirm readiness before start."),
    ).toBeTruthy();
    expect(screen.getByText("45s timer")).toBeTruthy();
    expect(screen.getByText("2 participants")).toBeTruthy();
    expect(screen.getByText("Child content")).toBeTruthy();
    expect(container.firstElementChild?.className).toContain("ready-room-shell");
  });

  it("omits optional lead copy and badges when they are not provided", () => {
    render(
      <ScreenShell title="Home">
        <div>Simple child</div>
      </ScreenShell>,
    );

    expect(screen.getByRole("heading", { name: "Home" })).toBeTruthy();
    expect(screen.queryByText("screen-lead")).toBeNull();
    expect(screen.getByText("Simple child")).toBeTruthy();
    expect(screen.queryByText("45s timer")).toBeNull();
  });
});