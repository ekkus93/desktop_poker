import { useEffect, useState } from "react";
import {
  House,
  RefreshCw,
  RotateCcw,
  TriangleAlert,
  WifiOff,
} from "lucide-react";
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
  | "invalid-invite"
  | "persistence-reset"
  | "invalid-lan-ip"
  | "join-failed";

const SCENARIO_LABELS: Record<ErrorScenario, string> = {
  reconnecting: "Reconnecting",
  "reconnect-success": "Reconnected",
  "reconnect-failed": "Reconnect failed",
  "host-lost": "Table unavailable",
  "invalid-invite": "Invalid invite",
  "persistence-reset": "Local data reset",
  "invalid-lan-ip": "Host setup blocked",
  "join-failed": "Join failed",
};

export function ErrorStateScreen({ bootstrap }: ScreenProps) {
  const { joinPayloadDraft, startupWarnings } = useDesktopShell();
  const [hostLanError, setHostLanError] = useState<string | null>(null);
  const [scenario, setScenario] = useState<ErrorScenario>(
    bootstrap.launchJoinPayloadError
      ? "invalid-invite"
      : startupWarnings.length > 0
        ? "persistence-reset"
        : "reconnecting",
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
          error instanceof Error
            ? error.message
            : "Unable to resolve a connectable LAN IP.";
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
    reconnecting: "Reconnecting to the table.",
    "reconnect-success": "Back at the table.",
    "reconnect-failed":
      "Could not restore this session. Rejoin if the game is still open.",
    "host-lost": "The host is gone or the table closed.",
    "invalid-invite": bootstrap.launchJoinPayloadError
      ? `Invite error: ${bootstrap.launchJoinPayloadError}`
      : "Invite error.",
    "persistence-reset":
      startupWarnings[0] ??
      "Some saved local data was unreadable and has been reset.",
    "invalid-lan-ip":
      hostLanError ?? "Hosting requires a reachable LAN IP address.",
    "join-failed": joinPayloadDraft.trim()
      ? "The host did not accept the connection."
      : "No valid invite was available.",
  };

  const scenarioAction = {
    reconnecting: {
      primaryLabel: "Back to lobby",
      primaryTo: "/lobby",
      secondaryLabel: "Return home",
      secondaryTo: "/",
    },
    "reconnect-success": {
      primaryLabel: "Open table",
      primaryTo: "/table",
      secondaryLabel: "Open history",
      secondaryTo: "/history",
    },
    "reconnect-failed": {
      primaryLabel: "Join tournament",
      primaryTo: "/join",
      secondaryLabel: "Return home",
      secondaryTo: "/",
    },
    "host-lost": {
      primaryLabel: "Open history",
      primaryTo: "/history",
      secondaryLabel: "Return home",
      secondaryTo: "/",
    },
    "invalid-invite": {
      primaryLabel: "Fix invite",
      primaryTo: "/join",
      secondaryLabel: "Return home",
      secondaryTo: "/",
    },
    "persistence-reset": {
      primaryLabel: "Return home",
      primaryTo: "/",
      secondaryLabel: "Open settings",
      secondaryTo: "/settings",
    },
    "invalid-lan-ip": {
      primaryLabel: "Open host setup",
      primaryTo: "/host",
      secondaryLabel: "Return home",
      secondaryTo: "/",
    },
    "join-failed": {
      primaryLabel: "Try joining again",
      primaryTo: "/join",
      secondaryLabel: "Return home",
      secondaryTo: "/",
    },
  }[scenario];
  const PrimaryIcon =
    scenario === "reconnecting" || scenario === "reconnect-success"
      ? RefreshCw
      : scenario === "host-lost"
        ? WifiOff
        : scenario === "invalid-invite" ||
            scenario === "invalid-lan-ip" ||
            scenario === "persistence-reset"
          ? TriangleAlert
          : RotateCcw;

  return (
    <ScreenShell
      title="Recovery"
      badges={[SCENARIO_LABELS[scenario]]}
      className="support-screen-shell"
    >
      <div className="recovery-grid">
        <SectionCard
          kicker="Current state"
          title={SCENARIO_LABELS[scenario]}
          className="support-card"
        >
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
              <span className="button-content">
                <PrimaryIcon className="button-icon" strokeWidth={1.9} />
                <span>{scenarioAction.primaryLabel}</span>
              </span>
            </Link>
            <Link className="secondary-button" to={scenarioAction.secondaryTo}>
              <span className="button-content">
                <House className="button-icon" strokeWidth={1.9} />
                <span>{scenarioAction.secondaryLabel}</span>
              </span>
            </Link>
          </div>
        </SectionCard>

        {bootstrap.debugToolsEnabled ? (
          <SectionCard
            kicker="Development only"
            title="Scenario picker"
            className="support-card"
          >
            <div className="button-row">
              {(Object.keys(SCENARIO_LABELS) as ErrorScenario[]).map(
                (candidate) => (
                  <button
                    className={
                      candidate === scenario
                        ? "primary-button compact-button"
                        : "secondary-button compact-button"
                    }
                    key={candidate}
                    onClick={() => {
                      setScenario(candidate);
                    }}
                    type="button"
                  >
                    {SCENARIO_LABELS[candidate]}
                  </button>
                ),
              )}
            </div>
          </SectionCard>
        ) : null}
      </div>
    </ScreenShell>
  );
}
