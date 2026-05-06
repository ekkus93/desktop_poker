import { useEffect, useMemo, useState } from "react";
import { ArrowRight, BadgeCheck, Clipboard, Link as LinkIcon, RotateCcw, TriangleAlert } from "lucide-react";
import { useLocation, useNavigate } from "react-router-dom";
import { useDesktopShell } from "../app/useDesktopShell";
import {
  joinHostSession,
  type JoinPayload,
  validateJoinPayloadInput,
} from "../api/desktop";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

type ValidationState =
  | { status: "idle" }
  | { status: "validating" }
  | { status: "valid"; payload: JoinPayload }
  | { status: "invalid"; message: string };

function extractCompactInvite(value: string) {
  return value.match(/\bpkr1_[A-Za-z0-9_-]+\b/)?.[0] ?? null;
}

function looksLikeHostShareDetails(value: string) {
  return value.includes("Tournament:") && value.includes("Host endpoint:");
}

function normaliseError(error: unknown) {
  return error instanceof Error ? error.message : "The invite could not be checked.";
}

export function JoinTournamentScreen({ bootstrap }: ScreenProps) {
  const {
    displayName,
    joinPayloadDraft,
    recentJoinPayloads,
    rememberJoinPayload,
    setJoinPayloadDraft,
    clearRecentJoinPayloads,
  } = useDesktopShell();
  const location = useLocation();
  const navigate = useNavigate();
  const [validationState, setValidationState] = useState<ValidationState>({
    status: "idle",
  });
  const [inviteBanner, setInviteBanner] = useState<string | null>(null);
  const [joinError, setJoinError] = useState<string | null>(null);
  const [joining, setJoining] = useState(false);

  const deepLinkPayload = useMemo(() => {
    const searchParams = new URLSearchParams(location.search);
    return searchParams.get("payload");
  }, [location.search]);

  useEffect(() => {
    if (deepLinkPayload && deepLinkPayload !== joinPayloadDraft) {
      setJoinPayloadDraft(deepLinkPayload);

      void validateJoinPayloadInput(deepLinkPayload)
        .then((parsedPayload) => {
          setValidationState({ status: "valid", payload: parsedPayload });
          setInviteBanner("Invite imported from a deep-link launch.");
        })
        .catch((error: unknown) => {
          setValidationState({
            status: "invalid",
            message: normaliseError(error),
          });
          setInviteBanner(null);
        });
    }
  }, [deepLinkPayload, joinPayloadDraft, setJoinPayloadDraft]);

  useEffect(() => {
    if (joinPayloadDraft !== bootstrap.launchJoinPayload) {
      return;
    }

    if (bootstrap.parsedLaunchJoinPayload) {
      setValidationState({
        status: "valid",
        payload: bootstrap.parsedLaunchJoinPayload,
      });
      setInviteBanner("Invite already attached to this launch.");
      return;
    }

    if (bootstrap.launchJoinPayloadError) {
      setValidationState({
        status: "invalid",
        message: bootstrap.launchJoinPayloadError,
      });
      setInviteBanner(null);
    }
  }, [
    bootstrap.launchJoinPayload,
    bootstrap.launchJoinPayloadError,
    bootstrap.parsedLaunchJoinPayload,
    joinPayloadDraft,
  ]);

  const reviewInvite = async () => {
    const trimmedPayload = joinPayloadDraft.trim();
    if (!trimmedPayload) {
      setValidationState({
        status: "invalid",
        message: "Paste an invite to continue.",
      });
      setInviteBanner(null);
      return null;
    }

    if (
      trimmedPayload === bootstrap.launchJoinPayload &&
      bootstrap.parsedLaunchJoinPayload
    ) {
      setValidationState({
        status: "valid",
        payload: bootstrap.parsedLaunchJoinPayload,
      });
      setInviteBanner("Invite already attached to this launch.");
      return bootstrap.parsedLaunchJoinPayload;
    }

    const compactInvite = extractCompactInvite(trimmedPayload);
    if (!compactInvite && looksLikeHostShareDetails(trimmedPayload)) {
      setValidationState({
        status: "invalid",
        message:
          "That pasted text is host share details, not a compact pkr1_ invite. Copy a real pkr1_ invite before checking it.",
      });
      setInviteBanner(null);
      return null;
    }

    setValidationState({ status: "validating" });

    try {
      const parsedPayload = await validateJoinPayloadInput(compactInvite ?? trimmedPayload);
      setValidationState({ status: "valid", payload: parsedPayload });
      rememberJoinPayload(compactInvite ?? trimmedPayload);
      setJoinError(null);
      setInviteBanner(
        `Ready: ${parsedPayload.hostAddress}:${parsedPayload.hostPort}`,
      );
      return parsedPayload;
    } catch (error) {
      setValidationState({
        status: "invalid",
        message: normaliseError(error),
      });
      setInviteBanner(null);
      return null;
    }
  };

  const sourceSummary = deepLinkPayload
    ? "Invite opened from a link"
    : bootstrap.launchJoinPayload
      ? "Invite already attached"
      : "Paste an invite";
  const previewTableName =
    validationState.status === "valid"
      ? validationState.payload.tableName ?? validationState.payload.tableId
      : null;
  const canContinueToLobby = validationState.status === "valid";
  const canCheckInvite = joinPayloadDraft.trim().length > 0 && validationState.status !== "validating";
  const continueHint = canContinueToLobby
    ? joining
      ? "Joining the live host session..."
      : "Invite checked. Join when ready."
    : validationState.status === "validating"
      ? "Continue unlocks after the invite check finishes."
      : validationState.status === "invalid"
        ? "Fix the invite above before continuing to the lobby."
        : "Paste an invite, then check it to continue.";

  const continueToLobby = async () => {
    if (validationState.status !== "valid") {
      return;
    }

    setJoining(true);
    setJoinError(null);

    try {
      await joinHostSession({
        joinPayload: joinPayloadDraft.trim(),
        displayName,
      });
      navigate("/lobby");
    } catch (error) {
      setJoinError(normaliseError(error));
    } finally {
      setJoining(false);
    }
  };

  return (
    <ScreenShell
      title="Join Tournament"
      badges={[sourceSummary]}
      className="pregame-screen-shell"
    >
      <div className="pregame-workstation join-station-layout">
        <SectionCard kicker="Join station" title="Join" className="workstation-main-card">
          <div className="workstation-grid join-workstation-grid">
            <div className="form-grid compact-form-grid invite-input-panel">
              <label className="field">
                Invite
                <textarea
                  className="compact-invite-textarea"
                  onChange={(event) => {
                    setJoinPayloadDraft(event.target.value);
                    setValidationState({ status: "idle" });
                    setInviteBanner(null);
                  }}
                  rows={6}
                  value={joinPayloadDraft}
                />
              </label>
              <div className="button-row workstation-actions">
                <button
                  className="primary-button"
                  disabled={!canCheckInvite}
                  onClick={() => {
                    void reviewInvite();
                  }}
                  type="button"
                >
                  <span className="button-content">
                    <Clipboard className="button-icon" strokeWidth={1.9} />
                    <span>{validationState.status === "validating" ? "Checking invite" : "Check invite"}</span>
                  </span>
                </button>
                {canContinueToLobby ? (
                  <button
                    className="primary-button ghost-primary-button"
                    disabled={joining}
                    onClick={() => {
                      void continueToLobby();
                    }}
                    type="button"
                  >
                    <span className="button-content">
                      <ArrowRight className="button-icon" strokeWidth={1.9} />
                      <span>{joining ? "Joining lobby" : "Continue to lobby"}</span>
                    </span>
                  </button>
                ) : (
                  <button className="primary-button ghost-primary-button" disabled type="button">
                    <span className="button-content">
                      <ArrowRight className="button-icon" strokeWidth={1.9} />
                      <span>Continue to lobby</span>
                    </span>
                  </button>
                )}
              </div>
              <p className="field-hint">{continueHint}</p>
              {validationState.status === "validating" ? (
                <p className="inline-banner info">Checking the invite and host details…</p>
              ) : null}
              {validationState.status === "invalid" ? (
                <p className="inline-banner error"><TriangleAlert className="button-icon" strokeWidth={1.9} />{validationState.message}</p>
              ) : null}
              {joinError ? (
                <p className="inline-banner error"><TriangleAlert className="button-icon" strokeWidth={1.9} />{joinError}</p>
              ) : null}
              {inviteBanner ? <p className="inline-banner success"><BadgeCheck className="button-icon" strokeWidth={1.9} />{inviteBanner}</p> : null}
            </div>

            <div className="workstation-side-panel join-preview-panel">
              {validationState.status === "valid" ? (
                <section aria-label="Invite preview" className="invite-card compact-invite-card">
                  <p className="kicker">Invite looks good</p>
                  <h4>{previewTableName}</h4>
                  <div className="invite-stat-grid compact-invite-stat-grid">
                    <div>
                      <span className="invite-stat-label">Host</span>
                      <strong>
                        <span className="detail-value-with-icon"><LinkIcon className="button-icon" strokeWidth={1.9} />{validationState.payload.hostAddress}:{validationState.payload.hostPort}</span>
                      </strong>
                    </div>
                    <div>
                      <span className="invite-stat-label">Table</span>
                      <strong>{previewTableName}</strong>
                    </div>
                    <div>
                      <span className="invite-stat-label">Status</span>
                      <strong>Lobby ready</strong>
                    </div>
                  </div>
                </section>
              ) : (
                <section className="placeholder-panel compact-placeholder-panel">
                  <p className="kicker">Preview</p>
                  <h3>Invite details</h3>
                </section>
              )}

              {recentJoinPayloads.length > 0 ? (
                <section className="recent-rail">
                  <div className="recent-rail-header">
                    <p className="field-hint">Recent invites</p>
                    <button
                      className="secondary-button compact-button"
                      onClick={clearRecentJoinPayloads}
                      type="button"
                    >
                      <span className="button-content">
                        <RotateCcw className="button-icon" strokeWidth={1.9} />
                        <span>Clear saved</span>
                      </span>
                    </button>
                  </div>
                  <div className="recent-pill-row">
                    {recentJoinPayloads.map((payload) => (
                      <button
                        className="list-button recent-pill"
                        key={payload}
                        onClick={() => {
                          setJoinPayloadDraft(payload);
                          setValidationState({ status: "validating" });
                          setInviteBanner(null);

                          void validateJoinPayloadInput(payload)
                            .then((parsedPayload) => {
                              setValidationState({ status: "valid", payload: parsedPayload });
                              setInviteBanner("Loaded a recent invite.");
                            })
                            .catch((error: unknown) => {
                              setValidationState({
                                status: "invalid",
                                message: normaliseError(error),
                              });
                              setInviteBanner(null);
                            });
                        }}
                        type="button"
                      >
                        <span className="mono-value">{payload}</span>
                      </button>
                    ))}
                  </div>
                </section>
              ) : (
                <p className="field-hint">No invites saved.</p>
              )}
            </div>
          </div>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
