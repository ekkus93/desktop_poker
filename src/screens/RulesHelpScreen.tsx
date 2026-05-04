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
      title="Game Help"
      lead="Quick rules, joining basics, and the name this device uses at the table."
      badges={["Help"]}
      className="support-screen-shell"
    >
      <div className="help-grid">
        <SectionCard kicker="Rules" title="Table basics" className="support-card">
          <ul>
            <li>Single-table Sit 'n Go No-Limit Texas Hold'em</li>
            <li>2 to 10 players with equal starting stacks</li>
            <li>Everyone joins the same host on the local network</li>
            <li>Busted players stay to watch as observers</li>
          </ul>
        </SectionCard>

        <SectionCard title="How joining works" className="support-card">
          <ul>
            <li>The host shares an invite from the host screen.</li>
            <li>Paste that invite on Join Tournament.</li>
            <li>Check the table preview, then continue into the lobby.</li>
            <li>The host computer needs a reachable LAN address before others can join.</li>
          </ul>
        </SectionCard>

        <SectionCard kicker="This device" title="Name and saved table info" className="support-card device-settings-card">
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
            <p className="field-hint">This is the name other players see when you host or join from here.</p>
            <div className="button-row">
              <button className="secondary-button" onClick={resetHostDraft} type="button">
                Reset host setup
              </button>
              <button
                className="secondary-button"
                onClick={clearRecentJoinPayloads}
                type="button"
              >
                Clear saved invites ({recentJoinPayloads.length})
              </button>
            </div>
            <ul>
              <li>Your display name is saved for future games on this device.</li>
              <li>Host setup can be reset if you want a clean table setup.</li>
              <li>{recentJoinPayloads.length} saved invite{recentJoinPayloads.length === 1 ? "" : "s"} on this device.</li>
            </ul>
          </div>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
