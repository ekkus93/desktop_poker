import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { createBootstrap } from "../test/fixtures";
import { AppShell } from "./AppShell";

vi.mock("./useDesktopBootstrap", () => ({
  useDesktopBootstrap: vi.fn(),
}));

import { useDesktopBootstrap } from "./useDesktopBootstrap";

const mockedUseDesktopBootstrap = vi.mocked(useDesktopBootstrap);

describe("AppShell", () => {
  it("routes directly to the join screen", () => {
    mockedUseDesktopBootstrap.mockReturnValue({
      bootstrap: createBootstrap({
        screens: [
          { id: "home", title: "Home", route: "/", surface: "primary" },
          { id: "join", title: "Join", route: "/join", surface: "primary" },
        ],
      }),
      error: null,
      loading: false,
    });

    renderWithRoute("/join");

    expect(
      screen.getByRole("heading", { level: 2, name: "Join Tournament" }),
    ).toBeTruthy();
  });

  it("redirects unknown routes back to home", () => {
    mockedUseDesktopBootstrap.mockReturnValue({
      bootstrap: createBootstrap({
        screens: [
          { id: "home", title: "Home", route: "/", surface: "primary" },
          { id: "join", title: "Join", route: "/join", surface: "primary" },
        ],
      }),
      error: null,
      loading: false,
    });

    renderWithRoute("/does-not-exist");

    expect(
      screen.getByRole("heading", { level: 2, name: "Choose a table" }),
    ).toBeTruthy();
  });
});

function renderWithRoute(initialEntry: string) {
  return render(
    <MemoryRouter initialEntries={[initialEntry]}>
      <AppShell />
    </MemoryRouter>,
  );
}
