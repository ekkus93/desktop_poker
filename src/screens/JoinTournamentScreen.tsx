import { useEffect, useMemo, useState } from "react";
import { ArrowRight, BadgeCheck, Clipboard, Link as LinkIcon, RotateCcw, TriangleAlert } from "lucide-react";
import { Link } from "react-router-dom";
import { useLocation } from "react-router-dom";
import { useDesktopShell } from "../app/useDesktopShell";
import {
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

function normaliseError(error: unknown) {
  return error instanceof Error ? error.message : "The invite could not be checked.";
}

export function JoinTournamentScreen({ bootstrap }: ScreenProps) {
  const {
    joinPayloadDraft,
    recentJoinPayloads,
    rememberJoinPayload,
    setJoinPayloadDraft,
    clearRecentJoinPayloads,
  } = useDesktopShell();
  const location = useLocation();
  const [validationState, setValidationState] = useState<ValidationState>({
    status: "idle",
  });
  const [inviteBanner, setInviteBanner] = useState<string | null>(null);
  const [continueToLobby, setContinueToLobby] = useState(false);

  const deepLinkPayload = useMemo(() => {
    const searchParams = new URLSearchParams(location.search);
    return searchParams.get("payload");
  }, [location.search]);

  useEffect(() => {
    if (deepLinkPayload && deepLinkPayload !== joinPayloadDraft) {
      setJoinPayloadDraft(deepLinkPayload);
      setInviteBanner("Invite imported from a deep-link launch.");
      setContinueToLobby(false);
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
      return;
    }

    if (bootstrap.launchJoinPayloadError) {
      setValidationState({
        status: "invalid",
        message: bootstrap.launchJoinPayloadError,
      });
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
      return bootstrap.parsedLaunchJoinPayload;
    }

    setValidationState({ status: "validating" });

    try {
      const parsedPayload = await validateJoinPayloadInput(trimmedPayload);
      setValidationState({ status: "valid", payload: parsedPayload });
      rememberJoinPayload(trimmedPayload);
      setContinueToLobby(true);
      setInviteBanner(
        `Invite ready for ${parsedPayload.hostAddress}:${parsedPayload.hostPort}.`,
      );
      return parsedPayload;
    } catch (error) {
      setValidationState({
        status: "invalid",
        message: normaliseError(error),
      });
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

  return (
    <ScreenShell
      title="Join Tournament"
      lead="Paste the invite you were given, review the destination, then continue into the lobby."
      badges={[sourceSummary]}
    >
      <div className="content-grid">
        <SectionCard kicker="Step 1" title="Bring your invite">
          <div className="form-grid">
            <label className="field">
              Tournament invite
              <textarea
                onChange={(event) => {
                  setJoinPayloadDraft(event.target.value);
                  setValidationState({ status: "idle" });
                  setInviteBanner(null);
                  setContinueToLobby(false);
                }}
                rows={8}
                value={joinPayloadDraft}
              />
            </label>
            <div className="button-row">
              <button
                className="primary-button"
                onClick={() => {
                  void reviewInvite();
                }}
                type="button"
              >
                <span className="button-content">
                  <Clipboard className="button-icon" strokeWidth={1.9} />
                  <span>Review invite</span>
                </span>
              </button>
            </div>
            <p className="field-hint">
              Paste the invite you were given, or open the app from a join link.
            </p>
          </div>
          {validationState.status === "validating" ? (
            <p className="inline-banner info">Checking the invite…</p>
          ) : null}
          {validationState.status === "invalid" ? (
            <p className="inline-banner error"><TriangleAlert className="button-icon" strokeWidth={1.9} />{validationState.message}</p>
          ) : null}
          {inviteBanner ? <p className="inline-banner success"><BadgeCheck className="button-icon" strokeWidth={1.9} />{inviteBanner}</p> : null}
        </SectionCard>

        <SectionCard kicker="Step 2" title="Check the table">
          {validationState.status === "valid" ? (
            <section aria-label="Invite preview" className="invite-card">
              <p className="kicker">Invite looks good</p>
              <h4>{previewTableName}</h4>
              <p className="invite-lead">You are about to join this table from the lobby.</p>
              <div className="invite-stat-grid">
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
                  <strong>Ready for lobby check-in</strong>
                </div>
              </div>
            </section>
          ) : (
            <p>
              Invite details appear here before you continue.
            </p>
          )}
        </SectionCard>

        <SectionCard kicker="Step 3" title="Join the table">
          {continueToLobby ? (
            <div className="button-row">
              <Link className="primary-button" to="/lobby">
                <span className="button-content">
                  <ArrowRight className="button-icon" strokeWidth={1.9} />
                  <span>Continue to lobby</span>
                </span>
              </Link>
            </div>
          ) : (
            <p className="field-hint">Review the invite first. Once it looks right, the lobby action appears here.</p>
          )}
          {recentJoinPayloads.length > 0 ? (
            <div className="stacked-list">
              <p className="field-hint">Recent invites</p>
              {recentJoinPayloads.map((payload) => (
                <button
                  className="list-button"
                  key={payload}
                  onClick={() => {
                    setJoinPayloadDraft(payload);
                    setInviteBanner("Loaded a recent invite.");
                    setValidationState({ status: "idle" });
                    setContinueToLobby(false);
                  }}
                  type="button"
                >
                  <span className="mono-value">{payload}</span>
                </button>
              ))}
            </div>
          ) : (
            <p>No saved invites yet.</p>
          )}
          {recentJoinPayloads.length > 0 ? (
            <button
              className="secondary-button"
              onClick={clearRecentJoinPayloads}
              type="button"
            >
              <span className="button-content">
                <RotateCcw className="button-icon" strokeWidth={1.9} />
                <span>Clear recent invites</span>
              </span>
            </button>
          ) : null}
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
