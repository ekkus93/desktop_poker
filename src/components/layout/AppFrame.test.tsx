import { render } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import type { DesktopBootstrapState } from "../../api/desktop";
import { AppFrame } from "./AppFrame";

vi.mock("../../app/useDesktopShell", () => ({
  useDesktopShell: vi.fn(() => ({ displayName: "Test Player" })),
}));

describe("AppFrame", () => {
  it("locks the outer shell to the viewport and allows only inner content scrolling", () => {
    const bootstrap = { appName: "Test App", screens: [] } as DesktopBootstrapState;

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
});
