import { render } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";
import { createBootstrap } from "../../test/fixtures";
import { AppFrame } from "./AppFrame";

vi.mock("../../app/useDesktopShell", () => ({
  useDesktopShell: vi.fn(() => ({ displayName: "Test Player" })),
}));

describe("AppFrame", () => {
  it("locks the outer shell to the viewport and allows only inner content scrolling", () => {
    const bootstrap = createBootstrap({ appName: "Test App", screens: [] });

    const { container } = render(
      <MemoryRouter initialEntries={["/"]}>
        <AppFrame bootstrap={bootstrap} navigation={[]}>
          <div>Test content</div>
        </AppFrame>
      </MemoryRouter>,
    );

    const shell = container.querySelector(".app-frame");
    expect(shell).toBeTruthy();
    expect(shell).toBeInstanceOf(HTMLElement);
    expect((shell as HTMLElement).style.height).toBe("100dvh");
    expect((shell as HTMLElement).style.overflow).toBe("hidden");

    const content = shell?.querySelector(".content");
    expect(content).toBeTruthy();
    expect(content).toBeInstanceOf(HTMLElement);
    expect((content as HTMLElement).style.height).toBe("100%");
    expect((content as HTMLElement).style.overflow).toBe("hidden");
  });

  it("drops the support rail from the in-tournament shell and narrows the play frame", () => {
    const bootstrap = createBootstrap({ appName: "Test App", screens: [] });

    const { container, getByLabelText, getByRole, queryByText } = render(
      <MemoryRouter initialEntries={["/lobby"]}>
        <AppFrame
          bootstrap={bootstrap}
          navigation={[
            { to: "/", label: "Home" },
            { to: "/host", label: "Host" },
            { to: "/join", label: "Join" },
            { to: "/lobby", label: "Lobby" },
            { to: "/table", label: "Main Table" },
            { to: "/complete", label: "Complete" },
            { to: "/history", label: "History" },
            { to: "/rules", label: "Help" },
            { to: "/settings", label: "Settings" },
          ]}
        >
          <div>Lobby content</div>
        </AppFrame>
      </MemoryRouter>,
    );

    expect(container.querySelector(".tournament-frame")).toBeTruthy();
    expect(getByLabelText("Desktop poker support navigation")).toBeTruthy();
    expect(getByRole("link", { name: "History" })).toBeTruthy();
    expect(getByRole("link", { name: "Help" })).toBeTruthy();
    expect(getByRole("link", { name: "Settings" })).toBeTruthy();
    expect(queryByText("Lobby")).toBeNull();
    expect(queryByText("Main Table")).toBeNull();
    expect(queryByText("Complete")).toBeNull();
    expect(queryByText("Host")).toBeNull();
    expect(queryByText("Join")).toBeNull();
  });

  it("hides current-page host and lobby navigation buttons", () => {
    const bootstrap = createBootstrap({ appName: "Test App", screens: [] });
    const navigation = [
      { to: "/", label: "Home" },
      { to: "/host", label: "Host" },
      { to: "/join", label: "Join" },
      { to: "/lobby", label: "Lobby" },
      { to: "/history", label: "History" },
      { to: "/rules", label: "Help" },
      { to: "/settings", label: "Settings" },
    ];

    const hostRender = render(
      <MemoryRouter initialEntries={["/host"]}>
        <AppFrame bootstrap={bootstrap} navigation={navigation}>
          <div>Host content</div>
        </AppFrame>
      </MemoryRouter>,
    );

    expect(hostRender.queryByRole("link", { name: "Host" })).toBeNull();
    expect(hostRender.getByText("Host")).toBeTruthy();
    hostRender.unmount();

    const lobbyRender = render(
      <MemoryRouter initialEntries={["/lobby"]}>
        <AppFrame bootstrap={bootstrap} navigation={navigation}>
          <div>Lobby content</div>
        </AppFrame>
      </MemoryRouter>,
    );

    expect(lobbyRender.queryByRole("link", { name: "Lobby" })).toBeNull();
    expect(lobbyRender.getByText("Lobby content")).toBeTruthy();
  });
});
