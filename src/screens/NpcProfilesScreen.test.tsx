import { fireEvent, screen, waitFor } from "@testing-library/react";
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
    id: "test-player",
    name: "Test Player",
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

describe("NpcProfilesScreen", () => {
  beforeEach(() => {
    mockedListNpcProfiles.mockReset();
    mockedSaveNpcProfile.mockReset();
    mockedDeleteNpcProfile.mockReset();
    mockedListNpcProfiles.mockResolvedValue(profileList([]));
    mockedSaveNpcProfile.mockResolvedValue(createProfile());
    mockedDeleteNpcProfile.mockResolvedValue(undefined);
  });

  it("lists profiles returned by listNpcProfiles", async () => {
    mockedListNpcProfiles.mockResolvedValue(
      profileList([
        createProfile({ id: "alice", name: "Aggressive Alice" }),
        createProfile({ id: "bob", name: "Balanced Bob" }),
      ]),
    );
    const bootstrap = createBootstrap({ llmApiKeyConfigured: true });
    renderWithProviders(<NpcProfilesScreen />, { bootstrap });

    expect(await screen.findByText("Aggressive Alice")).toBeTruthy();
    expect(screen.getByText("Balanced Bob")).toBeTruthy();
  });

  it("clicking Save on a profile calls saveNpcProfile", async () => {
    const profile = createProfile({ id: "test-player", name: "Test Player" });
    mockedListNpcProfiles.mockResolvedValue(profileList([profile]));
    const bootstrap = createBootstrap({ llmApiKeyConfigured: true });
    renderWithProviders(<NpcProfilesScreen />, { bootstrap });

    await screen.findByText("Test Player");
    fireEvent.click(screen.getByRole("button", { name: /test player/i }));

    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(mockedSaveNpcProfile).toHaveBeenCalledWith(
        "test-player",
        expect.stringContaining("Test Player"),
      );
    });
  });

  it("clicking Delete on a non-builtin calls deleteNpcProfile", async () => {
    const profile = createProfile({ id: "my-custom", name: "My Custom" });
    mockedListNpcProfiles.mockResolvedValue(profileList([profile]));
    const bootstrap = createBootstrap({ llmApiKeyConfigured: true });
    renderWithProviders(<NpcProfilesScreen />, { bootstrap });

    await screen.findByText("My Custom");
    fireEvent.click(screen.getByRole("button", { name: /my custom/i }));
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));

    await waitFor(() => {
      expect(mockedDeleteNpcProfile).toHaveBeenCalledWith("my-custom");
    });
  });

  it("Delete button is disabled for built-in starter profiles", async () => {
    const profile = createProfile({
      id: "aggressive-alice",
      name: "Aggressive Alice",
    });
    mockedListNpcProfiles.mockResolvedValue(profileList([profile]));
    const bootstrap = createBootstrap({ llmApiKeyConfigured: true });
    renderWithProviders(<NpcProfilesScreen />, { bootstrap });

    await screen.findByText("Aggressive Alice");
    fireEvent.click(screen.getByRole("button", { name: /aggressive alice/i }));

    const deleteButton = screen.getByRole("button", { name: "Delete" });
    expect((deleteButton as HTMLButtonElement).disabled).toBe(true);
  });

  it("shows Parsed sections when opponentTendencies is set", async () => {
    const profile = createProfile({
      id: "alice",
      name: "Alice",
      opponentTendencies: "Bluff tight players on the river.",
    });
    mockedListNpcProfiles.mockResolvedValue(profileList([profile]));
    const bootstrap = createBootstrap({ llmApiKeyConfigured: true });
    renderWithProviders(<NpcProfilesScreen />, { bootstrap });

    await screen.findByText("Alice");
    fireEvent.click(screen.getByRole("button", { name: /alice/i }));

    expect(screen.getByTestId("parsed-sections")).toBeTruthy();
    expect(screen.getByText("Opponent tendencies")).toBeTruthy();
    expect(screen.getByText("Bluff tight players on the river.")).toBeTruthy();
  });

  it("hides Parsed sections when both optional fields are null", async () => {
    const profile = createProfile({
      id: "plain",
      name: "Plain",
      opponentTendencies: null,
      tiltBehaviour: null,
    });
    mockedListNpcProfiles.mockResolvedValue(profileList([profile]));
    const bootstrap = createBootstrap({ llmApiKeyConfigured: true });
    renderWithProviders(<NpcProfilesScreen />, { bootstrap });

    await screen.findByText("Plain");
    fireEvent.click(screen.getByRole("button", { name: /plain/i }));

    expect(screen.queryByTestId("parsed-sections")).toBeNull();
  });

  it("shows the Profile format help details block in the editor", async () => {
    const profile = createProfile({ id: "alice", name: "Alice" });
    mockedListNpcProfiles.mockResolvedValue(profileList([profile]));
    const bootstrap = createBootstrap({ llmApiKeyConfigured: true });
    renderWithProviders(<NpcProfilesScreen />, { bootstrap });

    await screen.findByText("Alice");
    fireEvent.click(screen.getByRole("button", { name: /alice/i }));

    expect(screen.getByText("Profile format help")).toBeTruthy();
  });

  it("shows corrupt profile file as a warning in the profile list", async () => {
    mockedListNpcProfiles.mockResolvedValue({
      profiles: [],
      errors: [{ filename: "bad.md", error: "missing frontmatter" }],
    });
    const bootstrap = createBootstrap({ llmApiKeyConfigured: true });
    renderWithProviders(<NpcProfilesScreen />, { bootstrap });

    expect(await screen.findByTestId("profile-list-errors")).toBeTruthy();
    expect(screen.getByText(/bad\.md/)).toBeTruthy();
  });
});
