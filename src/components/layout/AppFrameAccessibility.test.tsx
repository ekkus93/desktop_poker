import { fireEvent, screen, waitFor } from "@testing-library/react";
import { Route, Routes } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createBootstrap, renderWithProviders } from "../../test/fixtures";
import { AppFrame, type NavigationItem } from "./AppFrame";

const navigation: NavigationItem[] = [
  { to: "/", label: "Home" },
  { to: "/host", label: "Host" },
  { to: "/join", label: "Join" },
  { to: "/history", label: "History" },
  { to: "/rules", label: "Help" },
  { to: "/settings", label: "Settings" },
];

function FrameHarness() {
  const bootstrap = createBootstrap({ appName: "Desktop Poker Test" });
  return (
    <AppFrame bootstrap={bootstrap} navigation={navigation}>
      <Routes>
        <Route path="/" element={<h2>Home surface</h2>} />
        <Route path="/host" element={<h2>Host surface</h2>} />
      </Routes>
    </AppFrame>
  );
}

describe("AppFrame accessibility navigation", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "requestAnimationFrame",
      (callback: FrameRequestCallback) => {
        callback(0);
        return 1;
      },
    );
    vi.stubGlobal("cancelAnimationFrame", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("provides a skip link targeting the main content region", () => {
    renderWithProviders(<FrameHarness />, { initialEntries: ["/"] });

    const skipLink = screen.getByRole("link", {
      name: "Skip to main content",
    });
    expect(skipLink.getAttribute("href")).toBe("#main-content");
    const main = screen.getByRole("main");
    expect(main.id).toBe("main-content");
    expect(main.getAttribute("tabindex")).toBe("-1");
  });

  it("moves focus to main content after an SPA route change", async () => {
    renderWithProviders(<FrameHarness />, { initialEntries: ["/"] });

    fireEvent.click(screen.getByRole("link", { name: "Host" }));
    expect(await screen.findByText("Host surface")).toBeTruthy();

    await waitFor(() => {
      expect(document.activeElement).toBe(screen.getByRole("main"));
    });
  });
});
