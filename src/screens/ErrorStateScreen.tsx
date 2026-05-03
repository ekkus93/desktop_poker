import { useEffect, useState } from "react";
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
  "reconnect-success": "Reconnect success",
  "reconnect-failed": "Reconnect failed",
  "host-lost": "Host lost / table closed",
  "invalid-payload": "Invalid payload",
  "invalid-lan-ip": "Invalid LAN IP",
  "join-failed": "Join failure",
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
      "The client has lost the socket but still holds reconnect-safe identity material. Show an explicit reconnecting state until the backend confirms success or failure.",
    "reconnect-success":
      "Reconnect succeeded and the latest authoritative snapshot replaced any stale local view.",
    "reconnect-failed":
      "Reconnect failed. The user must be told clearly whether the table is closed or the reconnect token/session no longer matches.",
    "host-lost":
      "The host is gone or the table closed. The shell shows a blocking dialog instead of leaving the user in an ambiguous state.",
    "invalid-payload": bootstrap.launchJoinPayloadError
      ? `Launch payload validation failed: ${bootstrap.launchJoinPayloadError}`
      : "The pasted direct payload could not be decoded or failed runtime validation.",
    "invalid-lan-ip": hostLanError ?? "Hosting requires a non-loopback LAN IP address.",
    "join-failed": joinPayloadDraft.trim()
      ? "The join payload decoded, but the host rejected the connection or the protocol/session guard failed."
      : "Join failed because no valid payload was supplied.",
  };

  return (
    <ScreenShell
      title="Reconnect / Error States"
      lead="Explicit banners and dialogs now exist for reconnect progress, host-lost outcomes, invalid payloads, invalid LAN IPs, and join failure messaging."
      badges={[bootstrap.runtimeTransport, SCENARIO_LABELS[scenario]]}
    >
      <div className="content-grid">
        <SectionCard kicker="Scenario picker" title="Error and reconnect states">
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
          <p className="field-hint">
            Bootstrap-derived failures are selected automatically when present; otherwise the shell exposes each state for QA review.
          </p>
        </SectionCard>

        <SectionCard title="Active banner">
          <p
            className={`inline-banner ${
              scenario === "reconnect-success" ? "success" : scenario === "reconnecting" ? "info" : "error"
            }`}
          >
            {scenarioMessage[scenario]}
          </p>
        </SectionCard>
      </div>

      <section className="dialog-card">
        <p className="kicker">Dialog preview</p>
        <h3>{SCENARIO_LABELS[scenario]}</h3>
        <p>{scenarioMessage[scenario]}</p>
        <div className="button-row">
          <button className="primary-button" type="button">
            Acknowledge
          </button>
          <button className="secondary-button" type="button">
            Return home
          </button>
        </div>
      </section>
    </ScreenShell>
  );
}
