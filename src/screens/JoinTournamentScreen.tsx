import { useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowRight,
  BadgeCheck,
  Clipboard,
  House,
  Link as LinkIcon,
  RotateCcw,
  TriangleAlert,
} from "lucide-react";
import { useLocation, useNavigate } from "react-router";
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
  return error instanceof Error
    ? error.message
    : "The invite could not be checked.";
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
  const [pillMeta, setPillMeta] = useState<
    Map<string, { label: string; invalid?: boolean }>
  >(new Map());
  const launchJoinAttemptedForPayload = useRef<string | null>(null);
  const deepLinkImportedRef = useRef<string | null>(null);

  const deepLinkPayload = useMemo(() => {
    const searchParams = new URLSearchParams(location.search);
    return searchParams.get("payload");
  }, [location.search]);

  // Decode recent invite pills for display labels
  useEffect(() => {
    let cancelled = false;

    void Promise.all(
      recentJoinPayloads.map(async (payload) => {
        try {
          const parsed = await validateJoinPayloadInput(payload);
          const label = parsed.tableName
            ? `${parsed.tableName} · ${parsed.hostAddress}`
            : `${parsed.hostAddress}:${parsed.hostPort}`;
          return [payload, { label }] as const;
        } catch {
          const truncated =
            payload.length > 20 ? `${payload.slice(0, 20)}…` : payload;
          return [payload, { label: truncated, invalid: true }] as const;
        }
      }),
    ).then((entries) => {
      if (!cancelled) {
        setPillMeta(new Map(entries));
      }
    });

    return () => {
      cancelled = true;
    };
  }, [recentJoinPayloads]);

  useEffect(() => {
    if (!deepLinkPayload || deepLinkPayload === joinPayloadDraft) {
      return;
    }
    // Prevent re-importing the same payload if the user clears the field while the
    // URL update and draft reset are still being batched into separate renders.
    if (deepLinkImportedRef.current === deepLinkPayload) {
      return;
    }

    deepLinkImportedRef.current = deepLinkPayload;
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

  useEffect(() => {
    const launchJoinPayload = bootstrap.launchJoinPayload;
    if (!launchJoinPayload || !bootstrap.parsedLaunchJoinPayload) {
      return;
    }

    if (joinPayloadDraft !== launchJoinPayload) {
      return;
    }

    if (launchJoinAttemptedForPayload.current === launchJoinPayload) {
      return;
    }

    let cancelled = false;

    launchJoinAttemptedForPayload.current = launchJoinPayload;
    setJoining(true);
    setJoinError(null);
    setValidationState({
      status: "valid",
      payload: bootstrap.parsedLaunchJoinPayload,
    });
    setInviteBanner("Joining the attached invite now.");

    void joinHostSession({
      joinPayload: launchJoinPayload,
      displayName,
    })
      .then(() => {
        if (cancelled) {
          return;
        }

        rememberJoinPayload(launchJoinPayload);
        navigate("/lobby");
      })
      .catch((error: unknown) => {
        if (cancelled) {
          return;
        }

        setJoinError(normaliseError(error));
        setInviteBanner("Invite already attached to this launch.");
      })
      .finally(() => {
        if (cancelled) {
          return;
        }

        setJoining(false);
      });

    return () => {
      cancelled = true;
    };
  }, [
    bootstrap.launchJoinPayload,
    bootstrap.parsedLaunchJoinPayload,
    displayName,
    joinPayloadDraft,
    navigate,
    rememberJoinPayload,
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
          "That looks like host details — ask the host to share the invite code (starts with pkr1_) instead.",
      });
      setInviteBanner(null);
      return null;
    }

    setValidationState({ status: "validating" });

    try {
      const timeoutPromise = new Promise<never>((_, reject) => {
        window.setTimeout(() => {
          reject(
            new Error(
              "Validation timed out — check your connection and try again.",
            ),
          );
        }, 10_000);
      });
      const parsedPayload = await Promise.race([
        validateJoinPayloadInput(compactInvite ?? trimmedPayload),
        timeoutPromise,
      ]);
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
      ? (validationState.payload.tableName ?? validationState.payload.tableId)
      : null;
  const canContinueToLobby = validationState.status === "valid";
  const canCheckInvite =
    joinPayloadDraft.trim().length > 0 &&
    validationState.status !== "validating";
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
        <SectionCard
          kicker="Join station"
          title="Join"
          className="workstation-main-card"
        >
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
                    <span>
                      {validationState.status === "validating"
                        ? "Checking invite"
                        : "Check invite"}
                    </span>
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
                      <span>
                        {joining ? "Joining lobby" : "Continue to lobby"}
                      </span>
                    </span>
                  </button>
                ) : (
                  <button
                    className="primary-button ghost-primary-button"
                    disabled
                    type="button"
                  >
                    <span className="button-content">
                      <ArrowRight className="button-icon" strokeWidth={1.9} />
                      <span>Continue to lobby</span>
                    </span>
                  </button>
                )}
              </div>
              <p className="field-hint">{continueHint}</p>
              {validationState.status === "validating" ? (
                <p className="inline-banner info">
                  Checking the invite and host details…
                </p>
              ) : null}
              {validationState.status === "invalid" ? (
                <p className="inline-banner error">
                  <TriangleAlert className="button-icon" strokeWidth={1.9} />
                  {validationState.message}
                </p>
              ) : null}
              {joinError ? (
                <>
                  <p className="inline-banner error">
                    <TriangleAlert className="button-icon" strokeWidth={1.9} />
                    {joinError}
                  </p>
                  {launchJoinAttemptedForPayload.current ? (
                    <div className="button-row">
                      <button
                        className="secondary-button"
                        onClick={() => {
                          setJoinPayloadDraft("");
                          setValidationState({ status: "idle" });
                          setJoinError(null);
                          setInviteBanner(null);
                          navigate(location.pathname, { replace: true });
                        }}
                        type="button"
                      >
                        <span className="button-content">
                          <RotateCcw
                            className="button-icon"
                            strokeWidth={1.9}
                          />
                          <span>Clear and enter a different invite</span>
                        </span>
                      </button>
                      <button
                        className="secondary-button"
                        onClick={() => navigate("/")}
                        type="button"
                      >
                        <span className="button-content">
                          <House className="button-icon" strokeWidth={1.9} />
                          <span>Return home</span>
                        </span>
                      </button>
                    </div>
                  ) : null}
                </>
              ) : null}
              {inviteBanner ? (
                <p className="inline-banner success">
                  <BadgeCheck className="button-icon" strokeWidth={1.9} />
                  {inviteBanner}
                </p>
              ) : null}
            </div>

            <div className="workstation-side-panel join-preview-panel">
              {validationState.status === "valid" ? (
                <section
                  aria-label="Invite preview"
                  className="invite-card compact-invite-card"
                >
                  <p className="kicker">Invite looks good</p>
                  <h4>{previewTableName}</h4>
                  <div className="invite-stat-grid compact-invite-stat-grid">
                    <div>
                      <span className="invite-stat-label">Host</span>
                      <strong>
                        <span className="detail-value-with-icon">
                          <LinkIcon className="button-icon" strokeWidth={1.9} />
                          {validationState.payload.hostAddress}:
                          {validationState.payload.hostPort}
                        </span>
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
                    {recentJoinPayloads.map((payload) => {
                      const meta = pillMeta.get(payload);
                      const pillLabel = meta?.label ?? payload.slice(0, 20);
                      const isInvalid = meta?.invalid ?? false;
                      return (
                        <button
                          className={`list-button recent-pill${isInvalid ? " recent-pill-invalid" : ""}`}
                          key={payload}
                          onClick={() => {
                            setJoinPayloadDraft(payload);
                            setValidationState({ status: "validating" });
                            setInviteBanner(null);

                            void validateJoinPayloadInput(payload)
                              .then((parsedPayload) => {
                                setValidationState({
                                  status: "valid",
                                  payload: parsedPayload,
                                });
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
                          title={
                            isInvalid ? "This invite may be outdated" : payload
                          }
                          type="button"
                        >
                          {pillLabel}
                        </button>
                      );
                    })}
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
