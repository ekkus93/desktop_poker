import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { Route, Routes } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  deleteNpcProfile,
  listNpcProfiles,
  saveNpcProfile,
  type NpcProfile,
} from "../api/desktop";
import { createBootstrap, renderWithProviders } from "../test/fixtures";
import { NpcProfilesScreen } from "./NpcProfilesScreen";

vi.mock("../api/desktop", async () => {
  const actual =
    await vi.importActual<typeof import("../api/desktop")>("../api/desktop");
  return {
    ...actual,
    listNpcProfiles: vi.fn(),
    saveNpcProfile: vi.fn(),
    deleteNpcProfile: vi.fn(),
  };
});

const mockedListNpcProfiles = vi.mocked(listNpcProfiles);
const mockedSaveNpcProfile = vi.mocked(saveNpcProfile);
const mockedDeleteNpcProfile = vi.mocked(deleteNpcProfile);

function createProfile(overrides: Partial<NpcProfile> = {}): NpcProfile {
  return {
    id: "first-player",
    name: "First Player",
    style: "balanced",
    skill: "intermediate",
    description: "A test profile body.",
    opponentTendencies: null,
    tiltBehaviour: null,
    ...overrides,
  };
}

function profileList(profiles: NpcProfile[]) {
  return { profiles, errors: [] };
}

function renderProfiles() {
  return renderWithProviders(<NpcProfilesScreen />, {
    bootstrap: createBootstrap({ llmApiKeyConfigured: true }),
    initialEntries: ["/npc-profiles"],
  });
}

describe("NpcProfilesScreen integrity guards", () => {
  beforeEach(() => {
    localStorage.clear();
    mockedListNpcProfiles.mockReset();
    mockedSaveNpcProfile.mockReset();
    mockedDeleteNpcProfile.mockReset();
    mockedListNpcProfiles.mockResolvedValue(profileList([]));
    mockedSaveNpcProfile.mockResolvedValue(createProfile());
    mockedDeleteNpcProfile.mockResolvedValue(undefined);
  });

  it("blocks an in-screen profile switch until the user discards changes", async () => {
    const first = createProfile();
    const second = createProfile({
      id: "second-player",
      name: "Second Player",
    });
    mockedListNpcProfiles.mockResolvedValue(profileList([first, second]));
    renderProfiles();

    fireEvent.click(
      await screen.findByRole("button", { name: /first player/i }),
    );
    fireEvent.change(screen.getByLabelText("Profile content"), {
      target: { value: "changed but unsaved" },
    });
    expect(screen.getByText("Unsaved profile changes.")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: /second player/i }));
    expect(
      screen.getByRole("dialog", {
        name: "Discard unsaved profile changes?",
      }),
    ).toBeTruthy();
    expect(screen.getByRole("heading", { name: "First Player" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Keep editing" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByDisplayValue("changed but unsaved")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: /second player/i }));
    fireEvent.click(screen.getByRole("button", { name: "Discard changes" }));
    expect(
      await screen.findByRole("heading", { name: "Second Player" }),
    ).toBeTruthy();
    expect(screen.queryByDisplayValue("changed but unsaved")).toBeNull();
  });

  it("blocks data-router navigation until the user explicitly discards changes", async () => {
    const profile = createProfile();
    mockedListNpcProfiles.mockResolvedValue(profileList([profile]));
    const { router } = renderWithProviders(
      <Routes>
        <Route path="/npc-profiles" element={<NpcProfilesScreen />} />
        <Route path="/settings" element={<h1>Settings destination</h1>} />
      </Routes>,
      {
        bootstrap: createBootstrap({ llmApiKeyConfigured: true }),
        initialEntries: ["/npc-profiles"],
      },
    );

    fireEvent.click(
      await screen.findByRole("button", { name: /first player/i }),
    );
    fireEvent.change(screen.getByLabelText("Profile content"), {
      target: { value: "unsaved route change" },
    });

    let navigationPromise!: Promise<void>;
    act(() => {
      navigationPromise = router.navigate("/settings");
    });

    expect(screen.queryByText("Settings destination")).toBeNull();
    expect(screen.getByRole("dialog")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Discard changes" }));
    await act(async () => {
      await navigationPromise;
    });
    expect(await screen.findByText("Settings destination")).toBeTruthy();
  });

  it("prevents a hard unload while profile edits are dirty", async () => {
    const profile = createProfile();
    mockedListNpcProfiles.mockResolvedValue(profileList([profile]));
    renderProfiles();

    fireEvent.click(
      await screen.findByRole("button", { name: /first player/i }),
    );
    fireEvent.change(screen.getByLabelText("Profile content"), {
      target: { value: "unsaved close attempt" },
    });

    const event = new Event("beforeunload", { cancelable: true });
    const dispatched = window.dispatchEvent(event);
    expect(dispatched).toBe(false);
    expect(event.defaultPrevented).toBe(true);
  });

  it("rejects malformed profile IDs before calling the backend", async () => {
    renderProfiles();

    fireEvent.click(await screen.findByRole("button", { name: "New profile" }));
    fireEvent.change(screen.getByLabelText(/profile id/i), {
      target: { value: "../Escape_Profile" },
    });
    fireEvent.change(screen.getByLabelText("Profile content"), {
      target: { value: "---\nname: Escape\n---\nbody" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(
      await screen.findByText(/invalid profile id.*1–64 lowercase/i),
    ).toBeTruthy();
    expect(mockedSaveNpcProfile).not.toHaveBeenCalled();
  });

  it("rejects duplicate profile IDs before calling the backend", async () => {
    const existing = createProfile({ id: "existing-player" });
    mockedListNpcProfiles.mockResolvedValue(profileList([existing]));
    renderProfiles();

    fireEvent.click(await screen.findByRole("button", { name: "New profile" }));
    fireEvent.change(screen.getByLabelText(/profile id/i), {
      target: { value: "existing-player" },
    });
    fireEvent.change(screen.getByLabelText("Profile content"), {
      target: { value: "---\nname: Duplicate\n---\nbody" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(
      await screen.findByText(
        "A profile with ID 'existing-player' already exists.",
      ),
    ).toBeTruthy();
    expect(mockedSaveNpcProfile).not.toHaveBeenCalled();
  });

  it("clears the dirty baseline after a successful save", async () => {
    const profile = createProfile();
    const updated = createProfile({ name: "Updated Player" });
    mockedListNpcProfiles.mockResolvedValue(profileList([profile]));
    mockedSaveNpcProfile.mockResolvedValue(updated);
    renderProfiles();

    fireEvent.click(
      await screen.findByRole("button", { name: /first player/i }),
    );
    fireEvent.change(screen.getByLabelText("Profile content"), {
      target: { value: "---\nname: Updated Player\n---\nupdated body" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(mockedSaveNpcProfile).toHaveBeenCalled();
    });
    expect(await screen.findByText("Profile saved.")).toBeTruthy();
    expect(screen.queryByText("Unsaved profile changes.")).toBeNull();
  });
});
