import { useEffect, useState } from "react";
import type { ChangeEvent } from "react";
import { Link } from "react-router-dom";
import { useDesktopShell } from "../app/useDesktopShell";
import {
  BLIND_PRESETS,
  buildHostShareText,
  MAX_PLAYER_OPTIONS,
  STARTING_STACK_OPTIONS,
  TURN_TIMER_OPTIONS,
} from "../app/shell";
import { resolveHostLanAddress } from "../api/desktop";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

function clampPort(value: string, fallbackPort: number) {
  const parsedValue = Number.parseInt(value, 10);
  if (Number.isNaN(parsedValue)) {
    return fallbackPort;
  }

  return Math.max(1, Math.min(65535, parsedValue));
}

export function HostTournamentSetupScreen({ bootstrap }: ScreenProps) {
  const { hostDraft, updateHostDraft } = useDesktopShell();
  const [resolvedHostIp, setResolvedHostIp] = useState<string | null>(null);
  const [lanError, setLanError] = useState<string | null>(null);
  const [copyState, setCopyState] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    void resolveHostLanAddress()
      .then((ipAddress) => {
        if (!active) {
          return;
        }

        setResolvedHostIp(ipAddress);
        setLanError(null);
      })
      .catch((error: unknown) => {
        if (!active) {
          return;
        }

        setResolvedHostIp(null);
        setLanError(
          error instanceof Error ? error.message : "Unable to resolve a LAN host address.",
        );
      });

    return () => {
      active = false;
    };
  }, []);

  const shareText = buildHostShareText(
    bootstrap,
    hostDraft,
    resolvedHostIp,
    lanError,
  );

  const handleCopy = async (value: string, message: string) => {
    await navigator.clipboard.writeText(value);
    setCopyState(message);
  };

  const handleTextField =
    (field: "tournamentName") =>
    (event: ChangeEvent<HTMLInputElement>) => {
      updateHostDraft({ [field]: event.target.value });
    };

  return (
    <ScreenShell
      title="Host Tournament Setup"
      lead="Set the basics, confirm this computer can host, then share the invite."
      badges={[`Port ${hostDraft.hostPort}`, bootstrap.runtimeTransport]}
    >
      <div className="content-grid">
        <SectionCard kicker="Step 1" title="Tournament setup">
          <div className="form-grid two-column-grid">
            <label className="field">
              Tournament name
              <input
                onChange={handleTextField("tournamentName")}
                value={hostDraft.tournamentName}
              />
            </label>
            <label className="field">
              Max players
              <select
                onChange={(event) => {
                  updateHostDraft({ maxPlayers: Number.parseInt(event.target.value, 10) });
                }}
                value={hostDraft.maxPlayers}
              >
                {MAX_PLAYER_OPTIONS.map((option) => (
                  <option key={option} value={option}>
                    {option} players
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              Starting stack
              <select
                onChange={(event) => {
                  updateHostDraft({
                    startingStack: Number.parseInt(event.target.value, 10),
                  });
                }}
                value={hostDraft.startingStack}
              >
                {STARTING_STACK_OPTIONS.map((option) => (
                  <option key={option} value={option}>
                    {option} chips
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              Blind preset
              <select
                onChange={(event) => {
                  updateHostDraft({ blindPresetId: event.target.value });
                }}
                value={hostDraft.blindPresetId}
              >
                {BLIND_PRESETS.map((preset) => (
                  <option key={preset.id} value={preset.id}>
                    {preset.label} · {preset.summary}
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              Turn timer
              <select
                onChange={(event) => {
                  updateHostDraft({
                    turnTimerSeconds: Number.parseInt(event.target.value, 10),
                  });
                }}
                value={hostDraft.turnTimerSeconds}
              >
                {TURN_TIMER_OPTIONS.map((option) => (
                  <option key={option} value={option}>
                    {option} seconds
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              Host port
              <input
                min={1}
                onChange={(event) => {
                  updateHostDraft({
                    hostPort: clampPort(event.target.value, bootstrap.defaultHostPort),
                  });
                }}
                type="number"
                value={hostDraft.hostPort}
              />
            </label>
          </div>
        </SectionCard>

        <SectionCard kicker="Step 2" title="LAN details">
          <p className="field-hint">
            Other players need a reachable LAN address from this computer before they can join.
          </p>
          <div className="status-row">
            <div className={`status-pill ${lanError ? "danger" : resolvedHostIp ? "success" : "info"}`}>
              {lanError ? "Host address unavailable" : resolvedHostIp ? `Ready on ${resolvedHostIp}` : "Checking host address"}
            </div>
          </div>
          <div className="button-row">
            <button
              className="secondary-button"
              onClick={() => {
                updateHostDraft({ advancedOpen: !hostDraft.advancedOpen });
              }}
              type="button"
            >
              {hostDraft.advancedOpen ? "Hide advanced settings" : "Show advanced settings"}
            </button>
          </div>
          {hostDraft.advancedOpen ? (
            <div className="form-grid">
              <p>
                Hosting needs a real LAN address that other devices can reach.
              </p>
              <ul>
                <li>
                  <strong>Resolved LAN IP:</strong> {resolvedHostIp ?? "Pending lookup"}
                </li>
                <li>
                  <strong>Host port:</strong> {hostDraft.hostPort}
                </li>
                <li>
                  <strong>Discovery:</strong> Direct payload join only.
                </li>
              </ul>
              {lanError ? <p className="inline-banner error">{lanError}</p> : null}
            </div>
          ) : null}
        </SectionCard>

        <SectionCard kicker="Step 3" title="Share the invite">
          <label className="field">
            Join details
            <textarea readOnly rows={9} value={shareText} />
          </label>
          <div className="button-row">
            <button
              className="secondary-button"
              onClick={() => {
                void handleCopy(shareText, "Copied host share details.");
              }}
              type="button"
            >
              Copy share details
            </button>
            <button className="secondary-button" disabled type="button">
              Copy payload
            </button>
            <button className="secondary-button" disabled type="button">
              Show QR
            </button>
            <Link className="primary-button" to="/lobby">
              Continue to lobby
            </Link>
          </div>
          <p className="field-hint">
            Share details are ready now. The direct <code>pkr1_</code> payload stays disabled until the host runtime produces it.
          </p>
          {copyState ? <p className="inline-banner success">{copyState}</p> : null}
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
