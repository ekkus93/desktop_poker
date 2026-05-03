import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function RulesHelpScreen({ bootstrap }: ScreenProps) {
  const {
    displayName,
    setDisplayName,
    hostDraft,
    resetHostDraft,
    recentJoinPayloads,
    clearRecentJoinPayloads,
  } = useDesktopShell();

  return (
    <ScreenShell
      title="Rules / Help"
      lead="Review the frozen Sit 'n Go rules, networking constraints, and desktop settings that are already available in the shell."
      badges={[`Port ${bootstrap.defaultHostPort}`, "Rules + settings"]}
    >
      <div className="content-grid">
        <SectionCard kicker="Rules" title="MVP game model">
          <ul>
            <li>Single-table Sit 'n Go No-Limit Texas Hold'em</li>
            <li>2 to 10 players with equal starting stacks</li>
            <li>Host-authoritative LAN TCP gameplay and reconnect handling</li>
            <li>Eliminated players remain read-only observers</li>
          </ul>
        </SectionCard>

        <SectionCard title="Networking help">
          <ul>
            <li>Direct payload join is the canonical v1 path.</li>
            <li>Room-code discovery is not exposed until it is real.</li>
            <li>Production hosting must fail loudly when no LAN IP exists.</li>
            <li>Multiple instances can run without profile collisions.</li>
          </ul>
        </SectionCard>

        <SectionCard kicker="Settings" title="Local shell settings">
          <div className="form-grid" id="settings">
            <label className="field">
              Display name
              <input
                onChange={(event) => {
                  setDisplayName(event.target.value);
                }}
                value={displayName}
              />
            </label>
            <p className="field-hint">
              Stored locally under <span className="mono-value">{bootstrap.profileDirectory}</span>.
            </p>
            <div className="button-row">
              <button className="secondary-button" onClick={resetHostDraft} type="button">
                Reset host defaults
              </button>
              <button
                className="secondary-button"
                onClick={clearRecentJoinPayloads}
                type="button"
              >
                Clear recent payloads ({recentJoinPayloads.length})
              </button>
            </div>
            <ul>
              <li>
                Current host draft: {hostDraft.tournamentName} on port {hostDraft.hostPort}
              </li>
              <li>Instance id: {bootstrap.instanceId}</li>
              <li>Debug tools: {bootstrap.debugToolsEnabled ? "enabled" : "hidden"}</li>
            </ul>
          </div>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
