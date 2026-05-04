import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useDesktopShell } from "./useDesktopShell";
import { createBootstrap, renderWithProviders } from "../test/fixtures";

function StorageHarness() {
  const {
    displayName,
    joinPayloadDraft,
    recentJoinPayloads,
    setDisplayName,
    setJoinPayloadDraft,
    rememberJoinPayload,
  } = useDesktopShell();

  return (
    <div>
      <p>Display: {displayName}</p>
      <p>Draft: {joinPayloadDraft || "empty"}</p>
      <p>Recent: {recentJoinPayloads.join(",") || "empty"}</p>
      <button onClick={() => setDisplayName("Alice")} type="button">
        Set Alice
      </button>
      <button onClick={() => setDisplayName("Bob")} type="button">
        Set Bob
      </button>
      <button onClick={() => setJoinPayloadDraft("pkr1_alpha")} type="button">
        Draft Alpha
      </button>
      <button onClick={() => setJoinPayloadDraft("pkr1_beta")} type="button">
        Draft Beta
      </button>
      <button onClick={() => rememberJoinPayload("pkr1_alpha")} type="button">
        Remember Alpha
      </button>
      <button onClick={() => rememberJoinPayload("pkr1_beta")} type="button">
        Remember Beta
      </button>
    </div>
  );
}

describe("DesktopShellProvider", () => {
  it("isolates stored shell state by storage namespace", () => {
    localStorage.clear();

    const hostBootstrap = createBootstrap({
      instanceId: "host-a",
      instanceLabel: "Host A",
      storageNamespace: "desktop-poker:host-a",
      sessionIdentity: "desktop-session:host-a",
      reconnectNamespace: "desktop-reconnect:host-a",
      profileDirectory: "/home/test/desktop-poker/profiles/host-a",
    });
    const clientBootstrap = createBootstrap({
      instanceId: "client-b",
      instanceLabel: "Client B",
      storageNamespace: "desktop-poker:client-b",
      sessionIdentity: "desktop-session:client-b",
      reconnectNamespace: "desktop-reconnect:client-b",
      profileDirectory: "/home/test/desktop-poker/profiles/client-b",
    });

    const hostRender = renderWithProviders(<StorageHarness />, {
      bootstrap: hostBootstrap,
    });
    fireEvent.click(screen.getByRole("button", { name: "Set Alice" }));
    fireEvent.click(screen.getByRole("button", { name: "Draft Alpha" }));
    fireEvent.click(screen.getByRole("button", { name: "Remember Alpha" }));
    hostRender.unmount();

    const clientRender = renderWithProviders(<StorageHarness />, {
      bootstrap: clientBootstrap,
    });
    fireEvent.click(screen.getByRole("button", { name: "Set Bob" }));
    fireEvent.click(screen.getByRole("button", { name: "Draft Beta" }));
    fireEvent.click(screen.getByRole("button", { name: "Remember Beta" }));
    clientRender.unmount();

    renderWithProviders(<StorageHarness />, { bootstrap: hostBootstrap });
    expect(screen.getByText("Display: Alice")).toBeTruthy();
    expect(screen.getByText("Draft: pkr1_alpha")).toBeTruthy();
    expect(screen.getByText("Recent: pkr1_alpha")).toBeTruthy();

    expect(localStorage.getItem("desktop-poker:host-a:display-name")).toContain(
      "Alice",
    );
    expect(localStorage.getItem("desktop-poker:client-b:display-name")).toContain(
      "Bob",
    );
  });

  it("normalizes legacy host drafts so critical controls stay visible", () => {
    localStorage.clear();

    const bootstrap = createBootstrap({
      storageNamespace: "desktop-poker:legacy-host",
      instanceId: "legacy-host",
      instanceLabel: "legacy-host",
    });

    localStorage.setItem(
      "desktop-poker:legacy-host:host-draft",
      JSON.stringify({
        tournamentName: "Legacy Sit 'n Go",
        maxPlayers: 6,
        startingStack: 1500,
        blindPresetId: "standard",
        turnTimerSeconds: 30,
        hostPort: 43818,
        advancedOpen: false,
      }),
    );

    renderWithProviders(<StorageHarness />, { bootstrap });

    expect(localStorage.getItem("desktop-poker:legacy-host:host-draft")).toContain(
      '"tournamentName":"Legacy Sit \'n Go"',
    );
    expect(localStorage.getItem("desktop-poker:legacy-host:host-draft")).not.toContain(
      '"advancedOpen":false',
    );
  });
});
