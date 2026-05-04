import { useEffect, useMemo, useState } from "react";
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
  return error instanceof Error ? error.message : "Join payload validation failed.";
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
  const [connectBanner, setConnectBanner] = useState<string | null>(null);
  const [continueToLobby, setContinueToLobby] = useState(false);

  const deepLinkPayload = useMemo(() => {
    const searchParams = new URLSearchParams(location.search);
    return searchParams.get("payload");
  }, [location.search]);

  useEffect(() => {
    if (deepLinkPayload && deepLinkPayload !== joinPayloadDraft) {
      setJoinPayloadDraft(deepLinkPayload);
      setConnectBanner("Join payload imported from a deep-link launch.");
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

  const runValidation = async () => {
    const trimmedPayload = joinPayloadDraft.trim();
    if (!trimmedPayload) {
      setValidationState({
        status: "invalid",
        message: "Paste a pkr1_ payload or legacy JSON payload to continue.",
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
    ? "Deep-link payload detected"
    : bootstrap.launchJoinPayload
      ? "CLI or environment launch payload detected"
      : "Manual paste mode";

  return (
    <ScreenShell
      title="Join Tournament"
      lead="Paste the payload you were given, confirm it looks right, then continue into the lobby."
      badges={[bootstrap.joinPayloadEncoding, sourceSummary]}
    >
      <div className="content-grid">
        <SectionCard kicker="Step 1" title="Paste payload">
          <div className="form-grid">
            <label className="field">
              Paste payload
              <textarea
                onChange={(event) => {
                  setJoinPayloadDraft(event.target.value);
                  setValidationState({ status: "idle" });
                  setConnectBanner(null);
                  setContinueToLobby(false);
                }}
                rows={8}
                value={joinPayloadDraft}
              />
            </label>
            <div className="button-row">
              <button
                className="secondary-button"
                onClick={() => {
                  void runValidation();
                }}
                type="button"
              >
                Validate payload
              </button>
              <button
                className="primary-button"
                onClick={() => {
                  void runValidation().then((payload) => {
                    if (!payload) {
                      return;
                    }

                    rememberJoinPayload(joinPayloadDraft);
                    setContinueToLobby(true);
                    setConnectBanner(
                      `Payload validated for ${payload.hostAddress}:${payload.hostPort}.`,
                    );
                  });
                }}
                type="button"
              >
                Connect
              </button>
            </div>
            <p className="field-hint">
              You can paste a payload manually or arrive here with one already attached at launch.
            </p>
          </div>
          {validationState.status === "validating" ? (
            <p className="inline-banner info">Validating payload with the Rust decoder…</p>
          ) : null}
          {validationState.status === "invalid" ? (
            <p className="inline-banner error">{validationState.message}</p>
          ) : null}
          {connectBanner ? <p className="inline-banner success">{connectBanner}</p> : null}
        </SectionCard>

        <SectionCard kicker="Step 2" title="Check the destination">
          {validationState.status === "valid" ? (
            <dl className="detail-grid">
              <div>
                <dt>Host</dt>
                <dd>
                  {validationState.payload.hostAddress}:{validationState.payload.hostPort}
                </dd>
              </div>
              <div>
                <dt>Table</dt>
                <dd>{validationState.payload.tableName ?? validationState.payload.tableId}</dd>
              </div>
              <div>
                <dt>Session epoch</dt>
                <dd>{validationState.payload.sessionEpoch}</dd>
              </div>
              <div>
                <dt>Payload version</dt>
                <dd>{validationState.payload.payloadVersion}</dd>
              </div>
            </dl>
          ) : (
            <p>
              Valid payload details appear here before you continue.
            </p>
          )}
        </SectionCard>

        <SectionCard kicker="Step 3" title="Continue or reuse a recent payload">
          {continueToLobby ? (
            <div className="button-row">
              <Link className="primary-button" to="/lobby">
                Continue to lobby
              </Link>
            </div>
          ) : (
            <p className="field-hint">Validate the payload first. Once it passes, the lobby action appears here.</p>
          )}
          {recentJoinPayloads.length > 0 ? (
            <div className="stacked-list">
              {recentJoinPayloads.map((payload) => (
                <button
                  className="list-button"
                  key={payload}
                  onClick={() => {
                    setJoinPayloadDraft(payload);
                    setConnectBanner("Loaded a recent payload into the join form.");
                    setValidationState({ status: "idle" });
                  }}
                  type="button"
                >
                  <span className="mono-value">{payload}</span>
                </button>
              ))}
            </div>
          ) : (
            <p>No join payloads have been saved on this device yet.</p>
          )}
          {recentJoinPayloads.length > 0 ? (
            <button
              className="secondary-button"
              onClick={clearRecentJoinPayloads}
              type="button"
            >
              Clear recent payloads
            </button>
          ) : null}
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
