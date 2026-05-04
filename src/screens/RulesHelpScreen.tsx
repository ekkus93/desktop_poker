import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";

export function RulesHelpScreen() {
  return (
    <ScreenShell
      title="Help"
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

        <SectionCard title="Host flow" className="support-card">
          <ul>
            <li>Start on Host Tournament and confirm the LAN address is ready.</li>
            <li>Share the invite card with everyone joining the table.</li>
            <li>Open the lobby only after the host machine shows a reachable address.</li>
          </ul>
        </SectionCard>

        <SectionCard title="Join flow" className="support-card">
          <ul>
            <li>The host shares an invite from the host screen.</li>
            <li>Paste that invite on Join Tournament.</li>
            <li>Check the table preview, then continue into the lobby.</li>
            <li>The host computer needs a reachable LAN address before others can join.</li>
          </ul>
        </SectionCard>

        <SectionCard title="Observers and history" className="support-card">
          <ul>
            <li>Eliminated players remain connected as public observers.</li>
            <li>Hand History keeps saved summaries on this device for offline review.</li>
            <li>Settings stores your local name, host draft, and recent invites.</li>
          </ul>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
