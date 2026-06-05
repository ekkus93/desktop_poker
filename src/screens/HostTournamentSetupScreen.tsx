import { useEffect, useState } from "react";
import type { ChangeEvent } from "react";
import { ArrowRight, Copy, Radio, TriangleAlert, Wifi } from "lucide-react";
import { Link } from "react-router-dom";
import { useDesktopShell } from "../app/useDesktopShell";
import {
  BLIND_PRESETS,
  MAX_PLAYER_OPTIONS,
  STARTING_STACK_OPTIONS,
  TURN_TIMER_OPTIONS,
} from "../app/shell";
import {
  addNpcPlayers,
  getHostSessionStatus,
  listNpcProfiles,
  resolveHostLanAddress,
  startHostSession,
  type NpcProfile,
  type NpcStyle,
} from "../api/desktop";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

const NPC_BOT_NAMES = ["Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Theta", "Iota"];

function npcDisplayName(index: number): string {
  const suffix = NPC_BOT_NAMES[index] ?? String(index + 1);
  return `Bot ${suffix}`;
}

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
  const { displayName, hostDraft, setWasHost, updateHostDraft } = useDesktopShell();
  const [resolvedHostIp, setResolvedHostIp] = useState<string | null>(null);
  const [lanError, setLanError] = useState<string | null>(null);
  const [hostSession, setHostSession] = useState<Awaited<ReturnType<typeof getHostSessionStatus>>>(null);
  const [hostError, setHostError] = useState<string | null>(null);
  const [starting, setStarting] = useState(false);
  const [availableProfiles, setAvailableProfiles] = useState<NpcProfile[]>([]);
  const [copyState, setCopyState] = useState<string | null>(null);
  const [copyError, setCopyError] = useState<string | null>(null);
  const [fallbackInvite, setFallbackInvite] = useState<string | null>(null);
  const [nameError, setNameError] = useState<string | null>(null);
  const [portError, setPortError] = useState<string | null>(null);
  const [npcError, setNpcError] = useState<string | null>(null);

  useEffect(() => {
    if (bootstrap.llmApiKeyConfigured) {
      void listNpcProfiles().then(setAvailableProfiles).catch(() => {});
    }
  }, [bootstrap.llmApiKeyConfigured]);

  useEffect(() => {
    let active = true;

    void getHostSessionStatus()
      .then((status) => {
        if (!active) {
          return;
        }

        setHostSession(status);
      })
      .catch(() => {
        if (!active) {
          return;
        }

        setHostSession(null);
      });

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

  const blindPreset =
    BLIND_PRESETS.find((preset) => preset.id === hostDraft.blindPresetId) ??
    BLIND_PRESETS[0];
  const inviteReady = Boolean(hostSession);
  const canContinueToLobby = Boolean(hostSession);
  const blockedProgressMessage = lanError
    ? "Resolve the LAN address before continuing to the lobby."
    : "Start hosting before continuing to the lobby.";
  const copyBlockedMessage = lanError
    ? "Invite copy is unavailable until the LAN address issue is fixed."
    : "Start hosting before copying the live invite.";
  const summaryTournamentName = hostSession?.tournamentName ?? hostDraft.tournamentName;
  const summaryHostAddress = hostSession
    ? `${hostSession.advertisedHost}:${hostSession.hostPort}`
    : resolvedHostIp
      ? `${resolvedHostIp}:${hostDraft.hostPort}`
      : "Waiting for LAN";
  const summaryMaxPlayers = hostSession
    ? hostSession.activeSeatCount + hostSession.openSeatCount
    : hostDraft.maxPlayers;

  const handleStartHosting = async () => {
    if (!resolvedHostIp || starting) {
      return;
    }

    setStarting(true);
    setNpcError(null);
    try {
      const status = await startHostSession({
        hostAddress: resolvedHostIp,
        hostPort: hostDraft.hostPort,
        tournamentName: hostDraft.tournamentName,
        maxPlayers: hostDraft.maxPlayers,
        startingStack: hostDraft.startingStack,
        blindPresetId: hostDraft.blindPresetId,
        turnTimerSeconds: hostDraft.turnTimerSeconds,
        displayName,
      });
      setHostError(null);
      setCopyError(null);
      setWasHost(true);

      if (hostDraft.npcCount > 0) {
        try {
          const npcs = Array.from({ length: hostDraft.npcCount }, (_, i) => ({
            displayName: npcDisplayName(i),
            style: hostDraft.npcStyle as NpcStyle,
            profileId: hostDraft.npcProfileId ?? null,
          }));
          const npcStatus = await addNpcPlayers({ npcs });
          setHostSession(npcStatus);
        } catch (npcErr) {
          setNpcError(
            npcErr instanceof Error ? npcErr.message : "Unable to add NPC players.",
          );
          setHostSession(status);
        }
      } else {
        setHostSession(status);
      }
    } catch (error) {
      setHostError(
        error instanceof Error ? error.message : "Unable to start hosting.",
      );
      setHostSession(null);
    } finally {
      setStarting(false);
    }
  };

  const handleCopy = async (value: string, message: string) => {
    try {
      await navigator.clipboard.writeText(value);
      setCopyState(message);
      setCopyError(null);
      setFallbackInvite(null);
    } catch {
      setCopyState(null);
      setCopyError("Copy failed. Share the invite manually.");
      setFallbackInvite(value);
    }
  };

  const handleCopyInvite = async () => {
    if (!hostSession) {
      return;
    }

    try {
      await handleCopy(hostSession.invite, "Copied invite.");
    } catch (error) {
      setCopyState(null);
      setFallbackInvite(null);
      setCopyError(
        error instanceof Error ? error.message : "Unable to generate an invite.",
      );
    }
  };

  const tournamentNameIsBlank = hostDraft.tournamentName.trim().length === 0;
  const canStartHosting = Boolean(resolvedHostIp) && !lanError && !starting && !tournamentNameIsBlank;

  const handleTextField =
    (field: "tournamentName") =>
    (event: ChangeEvent<HTMLInputElement>) => {
      updateHostDraft({ [field]: event.target.value });
      if (event.target.value.trim().length > 0) {
        setNameError(null);
      }
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
                    onBlur={() => {
                      if (hostDraft.tournamentName.trim().length === 0) {
                        setNameError("Name is required.");
                      }
                    }}
                    onChange={handleTextField("tournamentName")}
                    value={hostDraft.tournamentName}
                  />
                  {nameError ? <span className="field-hint error-hint">{nameError}</span> : null}
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
                      const raw = event.target.value;
                      const parsed = Number.parseInt(raw, 10);
                      const clamped = clampPort(raw, bootstrap.defaultHostPort);
                      updateHostDraft({ hostPort: clamped });
                      if (!raw || Number.isNaN(parsed) || parsed < 1 || parsed > 65535) {
                        setPortError("Port must be between 1 and 65535.");
                      } else {
                        setPortError(null);
                      }
                    }}
                    type="number"
                    value={hostDraft.hostPort}
                  />
                  {portError ? <span className="field-hint error-hint">{portError}</span> : null}
                </label>
              </div>
              <div className="setup-grid-row compact-advanced-panel">
                <label className="field">
                  NPC players
                  <select
                    onChange={(event) => {
                      updateHostDraft({
                        npcCount: Number.parseInt(event.target.value, 10),
                      });
                    }}
                    value={hostDraft.npcCount}
                  >
                    {Array.from({ length: hostDraft.maxPlayers }, (_, i) => i).map((count) => (
                      <option key={count} value={count}>
                        {count === 0 ? "None" : `${count} bot${count === 1 ? "" : "s"}`}
                      </option>
                    ))}
                  </select>
                </label>
                {hostDraft.npcCount > 0 ? (
                  <label className="field">
                    NPC style
                    <select
                      onChange={(event) => {
                        updateHostDraft({
                          npcStyle: event.target.value as "aggressive" | "conservative",
                        });
                      }}
                      value={hostDraft.npcStyle}
                    >
                      <option value="aggressive">Aggressive</option>
                      <option value="conservative">Conservative</option>
                    </select>
                  </label>
                ) : null}
                {hostDraft.npcCount > 0 && bootstrap.llmApiKeyConfigured ? (
                  <label className="field">
                    AI profile
                    <select
                      onChange={(event) => {
                        updateHostDraft({
                          npcProfileId: event.target.value || null,
                        });
                      }}
                      value={hostDraft.npcProfileId ?? ""}
                    >
                      <option value="">Rule-based (no profile)</option>
                      {availableProfiles.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.name}
                        </option>
                      ))}
                    </select>
                  </label>
                ) : null}
                {hostDraft.npcCount > 0 && !bootstrap.llmApiKeyConfigured ? (
                  <p className="field-hint">
                    Add a Claude API key in settings to use AI profiles.
                  </p>
                ) : null}
              </div>
              {lanError ? <p className="inline-banner error">{lanError}</p> : null}
              {npcError ? <p className="inline-banner error">{npcError}</p> : null}
            </div>

            <div className="workstation-side-panel">
              <section className="compact-status-panel host-summary-panel" aria-label="Host share summary">
                <div className="status-row">
                  <div
                    className={`status-pill ${lanError ? "danger" : hostSession ? "success" : resolvedHostIp ? "info" : "info"}`}
                  >
                    {lanError ? (
                      <TriangleAlert className="button-icon" strokeWidth={1.9} />
                    ) : hostSession ? (
                      <Radio className="button-icon" strokeWidth={1.9} />
                    ) : (
                      <Wifi className="button-icon" strokeWidth={1.9} />
                    )}
                    {lanError
                      ? "Hosting is blocked"
                      : hostSession
                        ? `Live on ${hostSession.advertisedHost}`
                        : resolvedHostIp
                          ? `Ready on ${resolvedHostIp}`
                        : "Checking LAN address"}
                  </div>
                </div>
                <div className="host-summary-grid">
                  <div>
                    <span className="invite-stat-label">Table</span>
                    <strong>{summaryTournamentName}</strong>
                  </div>
                  <div>
                    <span className="invite-stat-label">Players</span>
                    <strong>{summaryMaxPlayers} max</strong>
                  </div>
                  <div>
                    <span className="invite-stat-label">Host</span>
                    <strong>{summaryHostAddress}</strong>
                  </div>
                  <div>
                    <span className="invite-stat-label">Blinds</span>
                    <strong>{blindPreset.firstLevel}</strong>
                  </div>
                </div>
              </section>

              <div className="button-row workstation-actions">
                <button
                  className="primary-button compact-button"
                  disabled={!canStartHosting}
                  onClick={() => {
                    void handleStartHosting();
                  }}
                  type="button"
                >
                  <span className="button-content">
                    <Radio className="button-icon" strokeWidth={1.9} />
                    <span>{starting ? "Starting…" : hostSession ? "Restart hosting" : "Start hosting"}</span>
                  </span>
                </button>
                <button
                  className="secondary-button compact-button"
                  disabled={!inviteReady}
                  onClick={() => {
                    void handleCopyInvite();
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
              {hostError ? <p className="inline-banner error">{hostError}</p> : null}
              {!inviteReady ? (
                <p className="field-hint">{copyBlockedMessage}</p>
              ) : null}
              {!canContinueToLobby ? (
                <p className="field-hint">{blockedProgressMessage}</p>
              ) : null}

              {copyError ? <p className="inline-banner error">{copyError}</p> : null}
              {fallbackInvite ? (
                <label className="field">
                  Invite
                  <textarea
                    className="compact-invite-textarea"
                    readOnly
                    rows={6}
                    value={fallbackInvite}
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
