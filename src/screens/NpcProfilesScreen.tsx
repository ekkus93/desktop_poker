import { useEffect, useState } from "react";
import {
  deleteNpcProfile,
  listNpcProfiles,
  saveNpcProfile,
  type NpcProfile,
  type NpcProfileError,
} from "../api/desktop";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";

const BUILTIN_PROFILE_IDS = [
  "aggressive-alice",
  "conservative-carlos",
  "balanced-sam",
];

const FORMAT_HELP = `---
name: My Player
style: loose-aggressive
skill: intermediate
---
General strategy description here.

## Opponent tendencies
(Optional) How to adjust strategy based on observed opponent stats.

## Tilt behaviour
(Optional) How this player behaves after a losing streak.`;

type ProfileDetailState = {
  id: string;
  name: string;
  content: string;
  opponentTendencies: string | null;
  tiltBehaviour: string | null;
};

export function NpcProfilesScreen() {
  const [profiles, setProfiles] = useState<NpcProfile[]>([]);
  const [profileErrors, setProfileErrors] = useState<NpcProfileError[]>([]);
  const [loading, setLoading] = useState(true);
  const [listError, setListError] = useState<string | null>(null);
  const [detail, setDetail] = useState<ProfileDetailState | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [newMode, setNewMode] = useState(false);
  const [newId, setNewId] = useState("");
  const [newContent, setNewContent] = useState("");
  const [newError, setNewError] = useState<string | null>(null);

  async function reload() {
    setLoading(true);
    setListError(null);
    try {
      const result = await listNpcProfiles();
      setProfiles(result.profiles);
      setProfileErrors(result.errors);
    } catch (e) {
      setListError(e instanceof Error ? e.message : "Failed to load profiles.");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void reload();
  }, []);

  function handleOpenProfile(profile: NpcProfile) {
    const rawContent = `---\nname: ${profile.name}\nstyle: ${profile.style}\nskill: ${profile.skill}\n---\n${profile.description}`;
    setDetail({
      id: profile.id,
      name: profile.name,
      content: rawContent,
      opponentTendencies: profile.opponentTendencies,
      tiltBehaviour: profile.tiltBehaviour,
    });
    setSaveError(null);
    setSaveSuccess(false);
    setDeleteError(null);
  }

  async function handleSaveDetail() {
    if (!detail) return;
    setSaveError(null);
    setSaveSuccess(false);
    try {
      const updated = await saveNpcProfile(detail.id, detail.content);
      setSaveSuccess(true);
      setDetail((prev) =>
        prev
          ? {
              ...prev,
              name: updated.name,
              opponentTendencies: updated.opponentTendencies,
              tiltBehaviour: updated.tiltBehaviour,
            }
          : null,
      );
      await reload();
    } catch (e) {
      setSaveError(e instanceof Error ? e.message : "Failed to save profile.");
    }
  }

  async function handleDelete(id: string) {
    setDeleteError(null);
    try {
      await deleteNpcProfile(id);
      setDetail(null);
      await reload();
    } catch (e) {
      setDeleteError(
        e instanceof Error ? e.message : "Failed to delete profile.",
      );
    }
  }

  async function handleCreateNew() {
    setNewError(null);
    const slug = newId.trim();
    if (!slug) {
      setNewError("Profile ID is required.");
      return;
    }
    try {
      await saveNpcProfile(slug, newContent);
      setNewMode(false);
      setNewId("");
      setNewContent("");
      await reload();
    } catch (e) {
      setNewError(e instanceof Error ? e.message : "Failed to create profile.");
    }
  }

  const hasParsedSections =
    detail &&
    (detail.opponentTendencies != null || detail.tiltBehaviour != null);

  return (
    <ScreenShell
      title="AI Profiles"
      badges={["NPC configuration"]}
      className="support-screen-shell"
    >
      <div className="help-grid">
        <SectionCard
          kicker="Profiles"
          title="Player profiles"
          className="support-card"
        >
          {profileErrors.length > 0 ? (
            <div
              className="inline-banner error"
              data-testid="profile-list-errors"
            >
              <strong>
                Warning: {profileErrors.length} profile file(s) could not be
                loaded:
              </strong>
              <ul style={{ margin: "0.25rem 0 0", padding: "0 0 0 1rem" }}>
                {profileErrors.map((e) => (
                  <li key={e.filename}>
                    <code>{e.filename}</code>: {e.error}
                  </li>
                ))}
              </ul>
            </div>
          ) : null}
          {loading ? (
            <p>Loading profiles…</p>
          ) : listError ? (
            <p className="inline-banner error">{listError}</p>
          ) : profiles.length === 0 ? (
            <p>No profiles found.</p>
          ) : (
            <ul style={{ listStyle: "none", padding: 0 }}>
              {profiles.map((p) => (
                <li key={p.id} style={{ marginBottom: "0.5rem" }}>
                  <button
                    className="secondary-button"
                    onClick={() => handleOpenProfile(p)}
                    type="button"
                  >
                    {p.name}
                    <span
                      style={{
                        marginLeft: "0.5rem",
                        opacity: 0.6,
                        fontSize: "0.85em",
                      }}
                    >
                      {p.style} · {p.skill}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
          <div className="button-row">
            <button
              className="primary-button"
              onClick={() => {
                setNewMode(true);
                setSaveSuccess(false);
              }}
              type="button"
            >
              New profile
            </button>
          </div>
        </SectionCard>

        {detail ? (
          <SectionCard
            kicker="Editing"
            title={detail.name}
            className="support-card"
          >
            <div className="form-grid">
              <label className="field">
                Profile content
                <textarea
                  rows={14}
                  value={detail.content}
                  onChange={(e) => {
                    setSaveSuccess(false);
                    setDetail((prev) =>
                      prev ? { ...prev, content: e.target.value } : null,
                    );
                  }}
                />
              </label>
            </div>

            <details style={{ marginTop: "0.75rem" }}>
              <summary
                style={{
                  cursor: "pointer",
                  fontWeight: 600,
                  fontSize: "0.9em",
                }}
              >
                Profile format help
              </summary>
              <pre
                style={{
                  fontSize: "0.78em",
                  background: "var(--surface-2, #f5f5f5)",
                  padding: "0.75rem",
                  borderRadius: "4px",
                  overflowX: "auto",
                  marginTop: "0.5rem",
                  whiteSpace: "pre-wrap",
                }}
              >
                {FORMAT_HELP}
              </pre>
            </details>

            {hasParsedSections ? (
              <details
                style={{ marginTop: "0.75rem" }}
                data-testid="parsed-sections"
              >
                <summary
                  style={{
                    cursor: "pointer",
                    fontWeight: 600,
                    fontSize: "0.9em",
                  }}
                >
                  Parsed sections
                </summary>
                <div style={{ marginTop: "0.5rem", fontSize: "0.88em" }}>
                  {detail.opponentTendencies != null ? (
                    <div style={{ marginBottom: "0.5rem" }}>
                      <strong>Opponent tendencies</strong>
                      <pre
                        style={{
                          background: "var(--surface-2, #f5f5f5)",
                          padding: "0.5rem",
                          borderRadius: "4px",
                          whiteSpace: "pre-wrap",
                          marginTop: "0.25rem",
                        }}
                      >
                        {detail.opponentTendencies}
                      </pre>
                    </div>
                  ) : null}
                  {detail.tiltBehaviour != null ? (
                    <div>
                      <strong>Tilt behaviour</strong>
                      <pre
                        style={{
                          background: "var(--surface-2, #f5f5f5)",
                          padding: "0.5rem",
                          borderRadius: "4px",
                          whiteSpace: "pre-wrap",
                          marginTop: "0.25rem",
                        }}
                      >
                        {detail.tiltBehaviour}
                      </pre>
                    </div>
                  ) : null}
                </div>
              </details>
            ) : null}

            <div className="button-row" style={{ marginTop: "1rem" }}>
              <button
                className="primary-button"
                onClick={handleSaveDetail}
                type="button"
              >
                Save
              </button>
              <button
                className="secondary-button"
                disabled={BUILTIN_PROFILE_IDS.includes(detail.id)}
                onClick={() => handleDelete(detail.id)}
                type="button"
              >
                Delete
              </button>
            </div>
            {saveSuccess ? (
              <p className="inline-banner success">Profile saved.</p>
            ) : null}
            {saveError ? (
              <p className="inline-banner error">{saveError}</p>
            ) : null}
            {deleteError ? (
              <p className="inline-banner error">{deleteError}</p>
            ) : null}
          </SectionCard>
        ) : null}

        {newMode ? (
          <SectionCard
            kicker="New profile"
            title="Create profile"
            className="support-card"
          >
            <div className="form-grid">
              <label className="field">
                Profile ID (slug, e.g. my-player)
                <input
                  value={newId}
                  onChange={(e) => setNewId(e.target.value)}
                />
              </label>
              <label className="field">
                Profile content
                <textarea
                  rows={14}
                  value={newContent}
                  onChange={(e) => setNewContent(e.target.value)}
                />
              </label>
            </div>
            <div className="button-row">
              <button
                className="primary-button"
                onClick={handleCreateNew}
                type="button"
              >
                Save
              </button>
              <button
                className="secondary-button"
                onClick={() => {
                  setNewMode(false);
                  setNewError(null);
                }}
                type="button"
              >
                Cancel
              </button>
            </div>
            {newError ? (
              <p className="inline-banner error">{newError}</p>
            ) : null}
          </SectionCard>
        ) : null}
      </div>
    </ScreenShell>
  );
}
