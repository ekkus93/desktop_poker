import { useEffect, useState } from "react";
import type { ChangeEvent } from "react";
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
      lead="Configure a real single-table Sit 'n Go host draft, inspect LAN diagnostics, and keep the share surface aligned with what the Rust runtime can actually advertise."
      badges={[`Port ${hostDraft.hostPort}`, bootstrap.runtimeTransport]}
    >
      <div className="content-grid">
        <SectionCard kicker="Host configuration" title="Tournament setup">
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

        <SectionCard title="Advanced hosting diagnostics">
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
                Production hosting requires a non-loopback LAN IP. The shell asks
                the Rust runtime for that address instead of inventing one.
              </p>
              <ul>
                <li>
                  <strong>Resolved LAN IP:</strong> {resolvedHostIp ?? "Pending lookup"}
                </li>
                <li>
                  <strong>Host port:</strong> {hostDraft.hostPort}
                </li>
                <li>
                  <strong>Runtime warning:</strong> Room-code discovery stays hidden.
                </li>
              </ul>
              {lanError ? <p className="inline-banner error">{lanError}</p> : null}
            </div>
          ) : null}
        </SectionCard>

        <SectionCard kicker="Join share" title="Join payload display">
          <label className="field">
            Join payload slot
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
          </div>
          <p className="field-hint">
            Copy payload stays disabled until the live Rust host runtime emits an
            actual <code>pkr1_</code> join payload for this table.
          </p>
          {copyState ? <p className="inline-banner success">{copyState}</p> : null}
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
