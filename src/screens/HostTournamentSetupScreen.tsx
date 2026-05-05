import { useEffect, useState } from "react";
import type { ChangeEvent } from "react";
import { ArrowRight, Copy, TriangleAlert, Wifi } from "lucide-react";
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

function describeBlindOpening(firstLevel: string) {
  const [smallBlind, bigBlind] = firstLevel.split("/").map((value) => value.trim());
  return `Small blind ${smallBlind} · big blind ${bigBlind}`;
}

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
  const [copyError, setCopyError] = useState<string | null>(null);
  const [showFallbackShareDetails, setShowFallbackShareDetails] = useState(false);

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

  useEffect(() => {
    if (!copyState) {
      return undefined;
    }

    const timeoutId = window.setTimeout(() => {
      setCopyState(null);
    }, 2400);

    return () => {
      window.clearTimeout(timeoutId);
    };
  }, [copyState]);

  const shareText = buildHostShareText(
    bootstrap,
    hostDraft,
    resolvedHostIp,
    lanError,
  );
  const blindPreset =
    BLIND_PRESETS.find((preset) => preset.id === hostDraft.blindPresetId) ??
    BLIND_PRESETS[0];
  const inviteReady = Boolean(resolvedHostIp && !lanError);
  const canContinueToLobby = inviteReady;
  const blockedProgressMessage = lanError
    ? "Resolve the LAN address before continuing to the lobby."
    : "Still checking this computer's LAN address. Copy and continue unlock when the address is ready.";
  const copyBlockedMessage = lanError
    ? "Invite copy is unavailable until the LAN address issue is fixed."
    : "Invite copy unlocks after the LAN address check finishes.";

  const handleCopy = async (value: string, message: string) => {
    try {
      await navigator.clipboard.writeText(value);
      setCopyState(message);
      setCopyError(null);
      setShowFallbackShareDetails(false);
    } catch {
      setCopyState(null);
      setCopyError("Copy failed. Share the invite details manually.");
      setShowFallbackShareDetails(true);
    }
  };

  const handleTextField =
    (field: "tournamentName") =>
    (event: ChangeEvent<HTMLInputElement>) => {
      updateHostDraft({ [field]: event.target.value });
    };

  return (
    <ScreenShell
      title="Host Tournament Setup"
      badges={[]}
      className="pregame-screen-shell"
    >
      <div className="pregame-workstation host-station-layout">
        <SectionCard
          kicker="Host station"
          title="Tournament setup"
          className="workstation-main-card"
        >
          <div className="workstation-grid">
            <div className="host-setup-form">
              <div className="setup-grid-row">
                <label className="field setup-grid-field-span-2">
                  Tournament name
                  <input
                    onChange={handleTextField("tournamentName")}
                    value={hostDraft.tournamentName}
                  />
                </label>
              </div>

              <div className="setup-grid-row compact-advanced-panel">
                <label className="field">
                  Max players
                  <select
                    onChange={(event) => {
                      updateHostDraft({
                        maxPlayers: Number.parseInt(event.target.value, 10),
                      });
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
              </div>

              <div className="setup-grid-row compact-advanced-panel">
                <label className="field setup-grid-field-span-2">
                  Blind preset
                  <select
                    onChange={(event) => {
                      updateHostDraft({ blindPresetId: event.target.value });
                    }}
                    value={hostDraft.blindPresetId}
                  >
                    {BLIND_PRESETS.map((preset) => (
                      <option key={preset.id} value={preset.id}>
                        {preset.label} · {describeBlindOpening(preset.firstLevel)} · {preset.summary}
                      </option>
                    ))}
                  </select>
                </label>
              </div>

              <div className="setup-grid-row compact-advanced-panel">
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
                        hostPort: clampPort(
                          event.target.value,
                          bootstrap.defaultHostPort,
                        ),
                      });
                    }}
                    type="number"
                    value={hostDraft.hostPort}
                  />
                </label>
              </div>
              {lanError ? <p className="inline-banner error">{lanError}</p> : null}
            </div>

            <div className="workstation-side-panel">
              <section className="compact-status-panel host-summary-panel" aria-label="Host share summary">
                <div className="status-row">
                  <div
                    className={`status-pill ${lanError ? "danger" : resolvedHostIp ? "success" : "info"}`}
                  >
                    {lanError ? (
                      <TriangleAlert className="button-icon" strokeWidth={1.9} />
                    ) : (
                      <Wifi className="button-icon" strokeWidth={1.9} />
                    )}
                    {lanError
                      ? "Hosting is blocked"
                      : resolvedHostIp
                        ? `Ready on ${resolvedHostIp}`
                        : "Checking LAN address"}
                  </div>
                </div>
                <div className="host-summary-grid">
                  <div>
                    <span className="invite-stat-label">Table</span>
                    <strong>{hostDraft.tournamentName}</strong>
                  </div>
                  <div>
                    <span className="invite-stat-label">Players</span>
                    <strong>{hostDraft.maxPlayers} max</strong>
                  </div>
                  <div>
                    <span className="invite-stat-label">Host</span>
                    <strong>{resolvedHostIp ? `${resolvedHostIp}:${hostDraft.hostPort}` : "Waiting for LAN"}</strong>
                  </div>
                  <div>
                    <span className="invite-stat-label">Blinds</span>
                    <strong>{blindPreset.firstLevel}</strong>
                  </div>
                </div>
              </section>

              <div className="button-row workstation-actions">
                <button
                  className="secondary-button compact-button"
                  disabled={!inviteReady}
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
                {canContinueToLobby ? (
                  <Link className="primary-button compact-button" to="/lobby">
                    <span className="button-content">
                      <ArrowRight className="button-icon" strokeWidth={1.9} />
                      <span>Continue to lobby</span>
                    </span>
                  </Link>
                ) : (
                  <button className="primary-button compact-button" disabled type="button">
                    <span className="button-content">
                      <ArrowRight className="button-icon" strokeWidth={1.9} />
                      <span>Continue to lobby</span>
                    </span>
                  </button>
                )}
              </div>
              {!inviteReady ? (
                <p className="field-hint">{copyBlockedMessage}</p>
              ) : null}
              {!canContinueToLobby ? (
                <p className="field-hint">{blockedProgressMessage}</p>
              ) : null}

              {copyError ? <p className="inline-banner error">{copyError}</p> : null}
              {showFallbackShareDetails ? (
                <label className="field">
                  Invite details
                  <textarea
                    className="compact-invite-textarea"
                    readOnly
                    rows={6}
                    value={shareText}
                  />
                </label>
              ) : null}
            </div>
          </div>
        </SectionCard>
        {copyState ? <p className="toast-banner success">{copyState}</p> : null}
      </div>
    </ScreenShell>
  );
}
