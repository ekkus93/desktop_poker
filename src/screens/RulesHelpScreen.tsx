import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";

export function RulesHelpScreen() {
  const {
    displayName,
    setDisplayName,
    resetHostDraft,
    recentJoinPayloads,
    clearRecentJoinPayloads,
  } = useDesktopShell();

  return (
    <ScreenShell
      title="Rules & Settings"
      lead="Review the game basics and update what this device remembers."
      badges={["Rules + settings"]}
    >
      <div className="content-grid">
        <SectionCard kicker="Rules" title="Table basics">
          <ul>
            <li>Single-table Sit 'n Go No-Limit Texas Hold'em</li>
            <li>2 to 10 players with equal starting stacks</li>
            <li>Everyone joins the same host on the local network</li>
            <li>Busted players stay to watch as observers</li>
          </ul>
        </SectionCard>

        <SectionCard title="How joining works">
          <ul>
            <li>Share an invite from the host setup screen.</li>
            <li>Paste that invite on Join Tournament.</li>
            <li>Hosting requires a reachable LAN IP.</li>
            <li>Room-code discovery is not available yet.</li>
          </ul>
        </SectionCard>

        <SectionCard kicker="Settings" title="Saved on this device">
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
            <p className="field-hint">Used whenever you host or join from this device.</p>
            <div className="button-row">
              <button className="secondary-button" onClick={resetHostDraft} type="button">
                Reset host defaults
              </button>
              <button
                className="secondary-button"
                onClick={clearRecentJoinPayloads}
                type="button"
              >
                Clear recent invites ({recentJoinPayloads.length})
              </button>
            </div>
            <ul>
              <li>Your display name is saved for future games.</li>
              <li>Host defaults can be reset if you want to start fresh.</li>
              <li>{recentJoinPayloads.length} recent invite{recentJoinPayloads.length === 1 ? "" : "s"} saved on this device.</li>
            </ul>
          </div>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
