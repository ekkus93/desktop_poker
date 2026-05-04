import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { resolveHostLanAddress } from "../api/desktop";
import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

type ErrorScenario =
  | "reconnecting"
  | "reconnect-success"
  | "reconnect-failed"
  | "host-lost"
  | "invalid-payload"
  | "invalid-lan-ip"
  | "join-failed";

const SCENARIO_LABELS: Record<ErrorScenario, string> = {
  reconnecting: "Reconnecting",
  "reconnect-success": "Reconnected",
  "reconnect-failed": "Reconnect failed",
  "host-lost": "Table unavailable",
  "invalid-payload": "Invalid payload",
  "invalid-lan-ip": "Host setup blocked",
  "join-failed": "Join failed",
};

export function ErrorStateScreen({ bootstrap }: ScreenProps) {
  const { joinPayloadDraft } = useDesktopShell();
  const [hostLanError, setHostLanError] = useState<string | null>(null);
  const [scenario, setScenario] = useState<ErrorScenario>(
    bootstrap.launchJoinPayloadError ? "invalid-payload" : "reconnecting",
  );

  useEffect(() => {
    let active = true;

    void resolveHostLanAddress()
      .then(() => {
        if (active) {
          setHostLanError(null);
        }
      })
      .catch((error: unknown) => {
        if (!active) {
          return;
        }

        const message =
          error instanceof Error ? error.message : "Unable to resolve a connectable LAN IP.";
        setHostLanError(message);
        if (!bootstrap.launchJoinPayloadError) {
          setScenario("invalid-lan-ip");
        }
      });

    return () => {
      active = false;
    };
  }, [bootstrap.launchJoinPayloadError]);

  const scenarioMessage: Record<ErrorScenario, string> = {
    reconnecting:
      "The connection dropped. The app is trying to restore the same session.",
    "reconnect-success":
      "You are back in the same session with the latest table state.",
    "reconnect-failed":
      "The app could not restore the same session. Rejoin if the game is still available.",
    "host-lost":
      "The host is gone or the table closed. You cannot keep playing from this screen.",
    "invalid-payload": bootstrap.launchJoinPayloadError
      ? `The launch payload failed validation: ${bootstrap.launchJoinPayloadError}`
      : "The payload could not be decoded. Check it and try again.",
    "invalid-lan-ip": hostLanError ?? "Hosting requires a reachable LAN IP address.",
    "join-failed": joinPayloadDraft.trim()
      ? "The payload decoded, but the host did not accept the connection."
      : "Join failed because there was no valid payload to use.",
  };

  const scenarioAction = {
    reconnecting: { primaryLabel: "Back to lobby", primaryTo: "/lobby", secondaryLabel: "Return home", secondaryTo: "/" },
    "reconnect-success": { primaryLabel: "Open table", primaryTo: "/table", secondaryLabel: "Open history", secondaryTo: "/history" },
    "reconnect-failed": { primaryLabel: "Join tournament", primaryTo: "/join", secondaryLabel: "Return home", secondaryTo: "/" },
    "host-lost": { primaryLabel: "Open history", primaryTo: "/history", secondaryLabel: "Return home", secondaryTo: "/" },
    "invalid-payload": { primaryLabel: "Fix payload", primaryTo: "/join", secondaryLabel: "Return home", secondaryTo: "/" },
    "invalid-lan-ip": { primaryLabel: "Open host setup", primaryTo: "/host", secondaryLabel: "Return home", secondaryTo: "/" },
    "join-failed": { primaryLabel: "Try joining again", primaryTo: "/join", secondaryLabel: "Return home", secondaryTo: "/" },
  }[scenario];

  return (
    <ScreenShell
      title="Connection & Recovery"
      lead="Explain what happened, what it means, and the next best recovery step."
      badges={[SCENARIO_LABELS[scenario]]}
    >
      <div className="content-grid">
        <SectionCard kicker="Current state" title={SCENARIO_LABELS[scenario]}>
          <p
            className={`inline-banner ${
              scenario === "reconnect-success"
                ? "success"
                : scenario === "reconnecting"
                  ? "info"
                  : "error"
            }`}
          >
            {scenarioMessage[scenario]}
          </p>
          <div className="button-row">
            <Link className="primary-button" to={scenarioAction.primaryTo}>
              {scenarioAction.primaryLabel}
            </Link>
            <Link className="secondary-button" to={scenarioAction.secondaryTo}>
              {scenarioAction.secondaryLabel}
            </Link>
          </div>
        </SectionCard>

        {bootstrap.debugToolsEnabled ? (
          <SectionCard kicker="Internal review" title="Scenario picker">
            <div className="button-row">
              {(Object.keys(SCENARIO_LABELS) as ErrorScenario[]).map((candidate) => (
                <button
                  className={candidate === scenario ? "primary-button compact-button" : "secondary-button compact-button"}
                  key={candidate}
                  onClick={() => {
                    setScenario(candidate);
                  }}
                  type="button"
                >
                  {SCENARIO_LABELS[candidate]}
                </button>
              ))}
            </div>
            <p className="field-hint">Use this only to review states during development and QA.</p>
          </SectionCard>
        ) : null}
      </div>
    </ScreenShell>
  );
}
