import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { DesktopShellProvider } from "./DesktopShellProvider";
import { useDesktopShell } from "./useDesktopShell";
import { createBootstrap } from "../test/fixtures";

function ShellHookProbe() {
  const {
    displayName,
    hostDraft,
    joinPayloadDraft,
    recentJoinPayloads,
    persistedHandHistoryCount,
    startupWarnings,
    lastEndedSession,
    setDisplayName,
    setLastEndedSession,
    updateHostDraft,
    resetHostDraft,
    setJoinPayloadDraft,
    rememberJoinPayload,
    clearRecentJoinPayloads,
  } = useDesktopShell();

  return (
    <div>
      <p>Display: {displayName}</p>
      <p>Host name: {hostDraft.tournamentName}</p>
      <p>Host players: {hostDraft.maxPlayers}</p>
      <p>Join draft: {joinPayloadDraft || "empty"}</p>
      <p>Recent: {recentJoinPayloads.join(",") || "empty"}</p>
      <p>History count: {persistedHandHistoryCount}</p>
      <p>Warnings: {startupWarnings.join(" | ") || "none"}</p>
      <p>
        Last session:{" "}
        {lastEndedSession
          ? `${lastEndedSession.role}:${lastEndedSession.tournamentName}`
          : "none"}
      </p>
      <button onClick={() => setDisplayName("Alice")} type="button">
        Set Alice
      </button>
      <button
        onClick={() =>
          updateHostDraft({ tournamentName: "Turbo Night", maxPlayers: 8 })
        }
        type="button"
      >
        Patch host
      </button>
      <button onClick={() => resetHostDraft()} type="button">
        Reset host
      </button>
      <button onClick={() => setJoinPayloadDraft("pkr1_beta")} type="button">
        Set join draft
      </button>
      <button
        onClick={() => rememberJoinPayload("  pkr1_alpha  ")}
        type="button"
      >
        Remember join
      </button>
      <button onClick={() => clearRecentJoinPayloads()} type="button">
        Clear recents
      </button>
      <button
        onClick={() =>
          setLastEndedSession({ role: "client", tournamentName: "Night Turbo" })
        }
        type="button"
      >
        Set last ended
      </button>
      <button onClick={() => setLastEndedSession(null)} type="button">
        Clear last ended
      </button>
    </div>
  );
}

function renderShellHook(bootstrap = createBootstrap()) {
  return render(
    <MemoryRouter>
      <DesktopShellProvider bootstrap={bootstrap}>
        <ShellHookProbe />
      </DesktopShellProvider>
    </MemoryRouter>,
  );
}

describe("useDesktopShell", () => {
  it("fails clearly when used outside the shell provider", () => {
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    expect(() => render(<ShellHookProbe />)).toThrow(
      "useDesktopShell must be used inside the shell provider.",
    );

    consoleError.mockRestore();
  });

  it("exposes the current shell state from the provider and restores persisted values", () => {
    const bootstrap = createBootstrap({
      instanceId: "host-a",
      instanceLabel: "Host A",
      storageNamespace: "desktop-poker:host-a",
      launchJoinPayload: "pkr1_launch",
    });

    localStorage.setItem(
      "desktop-poker:host-a:display-name",
      JSON.stringify("Saved Alice"),
    );
    localStorage.setItem(
      "desktop-poker:host-a:host-draft",
      JSON.stringify({
        tournamentName: "Saved Table",
        maxPlayers: 8,
        startingStack: 5000,
        blindPresetId: "turbo",
        turnTimerSeconds: 45,
        hostPort: 49000,
      }),
    );
    localStorage.setItem(
      "desktop-poker:host-a:join-draft",
      JSON.stringify("pkr1_saved"),
    );
    localStorage.setItem(
      "desktop-poker:host-a:recent-join-payloads",
      JSON.stringify(["pkr1_recent"]),
    );
    localStorage.setItem(
      "desktop-poker:host-a:hand-history-summaries",
      JSON.stringify({
        updatedAtMs: 1,
        entries: [
          {
            handNumber: 4,
            summary: "Alice won",
            potTotal: 120,
            winningPlayers: ["Alice"],
            eliminatedPlayers: [],
            boardCards: [],
          },
        ],
      }),
    );

    renderShellHook(bootstrap);

    expect(screen.getByText("Display: Saved Alice")).toBeTruthy();
    expect(screen.getByText("Host name: Saved Table")).toBeTruthy();
    expect(screen.getByText("Host players: 8")).toBeTruthy();
    expect(screen.getByText("Join draft: pkr1_saved")).toBeTruthy();
    expect(screen.getByText("Recent: pkr1_recent")).toBeTruthy();
    expect(screen.getByText("History count: 1")).toBeTruthy();
    expect(screen.getByText("Warnings: none")).toBeTruthy();
  });

  it("persists shell actions back into derived and stored state", () => {
    const bootstrap = createBootstrap({
      instanceId: "host-b",
      instanceLabel: "Host B",
      storageNamespace: "desktop-poker:host-b",
    });

    renderShellHook(bootstrap);

    fireEvent.click(screen.getByRole("button", { name: "Set Alice" }));
    fireEvent.click(screen.getByRole("button", { name: "Patch host" }));
    fireEvent.click(screen.getByRole("button", { name: "Set join draft" }));
    fireEvent.click(screen.getByRole("button", { name: "Remember join" }));

    expect(screen.getByText("Display: Alice")).toBeTruthy();
    expect(screen.getByText("Host name: Turbo Night")).toBeTruthy();
    expect(screen.getByText("Host players: 8")).toBeTruthy();
    expect(screen.getByText("Join draft: pkr1_beta")).toBeTruthy();
    expect(screen.getByText("Recent: pkr1_alpha")).toBeTruthy();

    expect(localStorage.getItem("desktop-poker:host-b:display-name")).toContain(
      "Alice",
    );
    expect(localStorage.getItem("desktop-poker:host-b:host-draft")).toContain(
      '"tournamentName":"Turbo Night"',
    );
    expect(localStorage.getItem("desktop-poker:host-b:join-draft")).toContain(
      "pkr1_beta",
    );

    fireEvent.click(screen.getByRole("button", { name: "Reset host" }));
    fireEvent.click(screen.getByRole("button", { name: "Clear recents" }));

    expect(
      screen.getByText("Host name: Desktop Sit 'n Go Host B"),
    ).toBeTruthy();
    expect(screen.getByText("Recent: empty")).toBeTruthy();
  });

  it("falls back cleanly when persisted host draft data is invalid", () => {
    const bootstrap = createBootstrap({
      instanceId: "legacy-host",
      instanceLabel: "Legacy Host",
      storageNamespace: "desktop-poker:legacy-host",
      defaultHostPort: 43818,
    });

    localStorage.setItem(
      "desktop-poker:legacy-host:host-draft",
      JSON.stringify({
        tournamentName: "   ",
        maxPlayers: 99,
        startingStack: 9999,
        blindPresetId: "wrong",
        turnTimerSeconds: 999,
        hostPort: 0,
      }),
    );

    renderShellHook(bootstrap);

    expect(
      screen.getByText("Host name: Desktop Sit 'n Go Legacy Host"),
    ).toBeTruthy();
    expect(screen.getByText("Host players: 6")).toBeTruthy();
    expect(
      localStorage.getItem("desktop-poker:legacy-host:host-draft"),
    ).toContain('"hostPort":43818');
    expect(screen.getByText("Warnings: none")).toBeTruthy();
  });

  it("surfaces unreadable persisted shell data as startup warnings", () => {
    const bootstrap = createBootstrap({
      instanceId: "broken-storage",
      instanceLabel: "Broken Storage",
      storageNamespace: "desktop-poker:broken-storage",
    });

    localStorage.setItem(
      "desktop-poker:broken-storage:display-name",
      "{not-json",
    );
    localStorage.setItem(
      "desktop-poker:broken-storage:hand-history-summaries",
      "{still-not-json",
    );

    renderShellHook(bootstrap);

    expect(screen.getByText("Display: Player Broken Storage")).toBeTruthy();
    expect(
      screen.getByText(
        /some saved local preferences were unreadable and were reset to safe defaults/i,
      ),
    ).toBeTruthy();
    expect(
      screen.getByText(
        /saved hand history was unreadable and has been ignored for this session/i,
      ),
    ).toBeTruthy();
  });

  it("keeps storage namespaced so shell state does not bleed across profiles", () => {
    const hostBootstrap = createBootstrap({
      instanceId: "host-a",
      instanceLabel: "Host A",
      storageNamespace: "desktop-poker:host-a",
    });
    const clientBootstrap = createBootstrap({
      instanceId: "client-b",
      instanceLabel: "Client B",
      storageNamespace: "desktop-poker:client-b",
    });

    const hostRender = renderShellHook(hostBootstrap);
    fireEvent.click(screen.getByRole("button", { name: "Set Alice" }));
    fireEvent.click(screen.getByRole("button", { name: "Remember join" }));
    hostRender.unmount();

    const clientRender = renderShellHook(clientBootstrap);
    expect(screen.getByText("Display: Player Client B")).toBeTruthy();
    expect(screen.getByText("Recent: empty")).toBeTruthy();
    clientRender.unmount();

    renderShellHook(hostBootstrap);
    expect(screen.getByText("Display: Alice")).toBeTruthy();
    expect(screen.getByText("Recent: pkr1_alpha")).toBeTruthy();
  });

  it("persists lastEndedSession and restores it on remount", () => {
    const bootstrap = createBootstrap({
      instanceId: "persist-les",
      instanceLabel: "Persist LES",
      storageNamespace: "desktop-poker:persist-les",
    });

    const { unmount } = renderShellHook(bootstrap);

    expect(screen.getByText("Last session: none")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Set last ended" }));
    expect(
      screen.getByText("Last session: client:Night Turbo"),
    ).toBeTruthy();
    expect(
      localStorage.getItem("desktop-poker:persist-les:last-ended-session"),
    ).toContain("Night Turbo");

    unmount();

    renderShellHook(bootstrap);
    expect(
      screen.getByText("Last session: client:Night Turbo"),
    ).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Clear last ended" }));
    expect(screen.getByText("Last session: none")).toBeTruthy();
  });
});
