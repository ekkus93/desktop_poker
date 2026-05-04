import { useEffect, useState } from "react";
import type { ChangeEvent } from "react";
import { ArrowRight, ChevronDown, ChevronUp, Copy, TriangleAlert, Wifi } from "lucide-react";
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
      lead="Set the table basics, share the invite, then head to the lobby."
      badges={[`Port ${hostDraft.hostPort}`]}
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
          <div className="status-row">
            <div className={`status-pill ${lanError ? "danger" : resolvedHostIp ? "success" : "info"}`}>
              {lanError ? <TriangleAlert className="button-icon" strokeWidth={1.9} /> : <Wifi className="button-icon" strokeWidth={1.9} />}
              {lanError ? "Hosting is blocked" : resolvedHostIp ? `Ready on ${resolvedHostIp}` : "Checking this computer"}
            </div>
          </div>
          <p className="field-hint">
            Players can join once this computer has a reachable LAN address.
          </p>
          <div className="button-row">
            <button
              className="secondary-button"
              onClick={() => {
                updateHostDraft({ advancedOpen: !hostDraft.advancedOpen });
              }}
              type="button"
            >
              <span className="button-content">
                {hostDraft.advancedOpen ? <ChevronUp className="button-icon" strokeWidth={1.9} /> : <ChevronDown className="button-icon" strokeWidth={1.9} />}
                <span>{hostDraft.advancedOpen ? "Hide advanced settings" : "Show advanced settings"}</span>
              </span>
            </button>
          </div>
          {hostDraft.advancedOpen ? (
            <div className="form-grid">
              <p>
                Share from a LAN address other players on this network can reach.
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
              <span className="button-content">
                <Copy className="button-icon" strokeWidth={1.9} />
                <span>Copy share details</span>
              </span>
            </button>
            <Link className="primary-button" to="/lobby">
              <span className="button-content">
                <ArrowRight className="button-icon" strokeWidth={1.9} />
                <span>Continue to lobby</span>
              </span>
            </Link>
          </div>
          <p className="field-hint">
            Share details are ready now. Direct payload and QR sharing can appear here later when the runtime exposes them.
          </p>
          {copyState ? <p className="inline-banner success">{copyState}</p> : null}
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
