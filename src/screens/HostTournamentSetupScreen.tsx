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
  const blindPreset = BLIND_PRESETS.find((preset) => preset.id === hostDraft.blindPresetId) ?? BLIND_PRESETS[0];
  const inviteReady = Boolean(resolvedHostIp && !lanError);
  const [hasCheckedAdvancedOpen, setHasCheckedAdvancedOpen] = useState(false);

  const handleCopy = async (value: string, message: string) => {
    await navigator.clipboard.writeText(value);
    setCopyState(message);
  };

  useEffect(() => {
    if (!hasCheckedAdvancedOpen && hostDraft.advancedOpen === false) {
      updateHostDraft({ advancedOpen: true });
      setHasCheckedAdvancedOpen(true);
    }
  }, [hasCheckedAdvancedOpen, hostDraft.advancedOpen, updateHostDraft]);

  const handleTextField =
    (field: "tournamentName") =>
    (event: ChangeEvent<HTMLInputElement>) => {
      updateHostDraft({ [field]: event.target.value });
    };

  return (
    <ScreenShell
      title="Host Tournament Setup"
      badges={[`Port ${hostDraft.hostPort}`]}
      className="pregame-screen-shell"
    >
      <div className="pregame-workstation host-station-layout">
        <SectionCard kicker="Host station" title="Tournament setup" className="workstation-main-card">
          <div className="workstation-grid">
            <div className="form-grid two-column-grid compact-form-grid">
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
            </div>

            <div className="workstation-side-panel">
              <section className="compact-status-panel">
                <div className="status-row">
                  <div className={`status-pill ${lanError ? "danger" : resolvedHostIp ? "success" : "info"}`}>
                    {lanError ? <TriangleAlert className="button-icon" strokeWidth={1.9} /> : <Wifi className="button-icon" strokeWidth={1.9} />}
                    {lanError ? "Hosting is blocked" : resolvedHostIp ? `Ready on ${resolvedHostIp}` : "Checking this computer"}
                  </div>
                </div>
              </section>

              {inviteReady ? (
                <section
                  className="invite-card compact-invite-card"
                  aria-label="Copy invite details"
                  role="button"
                  tabIndex={0}
                  onClick={() => {
                    void handleCopy(shareText, "Copied host share details.");
                  }}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      void handleCopy(shareText, "Copied host share details.");
                    }
                  }}
                >
                  <p className="kicker">Ready to share</p>
                  <p className="invite-lead">Click the invite card or use the copy button below.</p>
                  <h4>{hostDraft.tournamentName}</h4>
                  <div className="invite-stat-grid compact-invite-stat-grid">
                    <div>
                      <span className="invite-stat-label">Join</span>
                      <strong>{resolvedHostIp}:{hostDraft.hostPort}</strong>
                    </div>
                    <div>
                      <span className="invite-stat-label">Seats</span>
                      <strong>{hostDraft.maxPlayers} players</strong>
                    </div>
                    <div>
                      <span className="invite-stat-label">Stack</span>
                      <strong>{hostDraft.startingStack} chips</strong>
                    </div>
                    <div>
                      <span className="invite-stat-label">Blinds</span>
                      <strong>{blindPreset.label} · {blindPreset.firstLevel}</strong>
                    </div>
                  </div>
                </section>
              ) : (
                <p className={`inline-banner ${lanError ? "error" : "info"}`}>{shareText}</p>
              )}

              <div className="button-row workstation-actions">
                <button
                  className="secondary-button"
                  onClick={() => {
                    void handleCopy(shareText, "Copied host share details.");
                  }}
                  type="button"
                >
                  <span className="button-content">
                    <Copy className="button-icon" strokeWidth={1.9} />
                    <span>Copy invite</span>
                  </span>
                </button>
                <Link className="primary-button" to="/lobby">
                  <span className="button-content">
                    <ArrowRight className="button-icon" strokeWidth={1.9} />
                    <span>Continue to lobby</span>
                  </span>
                </Link>
              </div>

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
                    <span>{hostDraft.advancedOpen ? "Hide game options" : "Show game options"}</span>
                  </span>
                </button>
              </div>

              {hostDraft.advancedOpen ? (
                <div className="form-grid compact-advanced-panel">
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
                  <div>
                    <strong>Resolved LAN IP:</strong> {resolvedHostIp ?? "Pending lookup"}
                  </div>
                  {lanError ? <p className="inline-banner error">{lanError}</p> : null}
                </div>
              ) : null}

              {copyState ? <p className="inline-banner success">{copyState}</p> : null}
            </div>
          </div>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
